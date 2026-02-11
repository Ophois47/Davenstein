/*
Davenstein - by David Petnick
*/
use bevy::{
	asset::{
		io::{
			AssetReader, AssetReaderError, PathStream, Reader, SliceReader,
		},
	},
	prelude::*,
};
use futures_lite::stream;
use memmap2::Mmap;
use std::{
	collections::HashMap,
	fs::File,
	io,
	path::{Path, PathBuf},
	sync::Arc,
};

const MAGIC: [u8; 4] = *b"DVPK";
const VERSION: u32 = 1;

pub struct PakAssetsPlugin;

impl Plugin for PakAssetsPlugin {
	fn build(&self, app: &mut App) {
		use bevy::asset::AssetApp;
		use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};

		if !should_use_pak() {
			return;
		}

		let Some(pak_path) = resolve_pak_path() else {
			warn!("pak enabled but no pak path resolved");
			return;
		};

		if !pak_path.exists() {
			warn!("assets.pak not found at {}", pak_path.display());
			return;
		}

		info!("using assets.pak {}", pak_path.display());

		let inner = match PakAssetReader::open(pak_path.as_path()) {
			Ok(r) => r.inner,
			Err(err) => {
				warn!("failed to open pak {}: {}", pak_path.display(), err);
				return;
			}
		};

		app.register_asset_source(
			AssetSourceId::Default,
			AssetSourceBuilder::new(move || Box::new(PakAssetReader { inner: inner.clone() })),
		);
	}
}

fn resolve_pak_path() -> Option<PathBuf> {
	if let Some(p) = std::env::var_os("DAVENSTEIN_PAK_PATH") {
		return Some(PathBuf::from(p));
	}

	default_pak_path()
}

fn should_use_pak() -> bool {
	#[cfg(debug_assertions)]
	{
		std::env::var_os("DAVENSTEIN_USE_PAK").is_some()
	}

	#[cfg(not(debug_assertions))]
	{
		true
	}
}

fn default_pak_path() -> Option<PathBuf> {
	let exe = std::env::current_exe().ok()?;
	let dir = exe.parent()?;
	Some(dir.join("assets.pak"))
}

#[derive(Clone)]
struct PakEntry {
	offset: u64,
	len: u64,
}

struct PakInner {
	_mmap_file: File,
	mmap: Mmap,
	files: HashMap<Box<str>, PakEntry>,
	dirs: HashMap<Box<str>, Vec<PathBuf>>,
}

struct PakAssetReader {
	inner: Arc<PakInner>,
}

impl PakAssetReader {
	fn open(path: &Path) -> io::Result<Self> {
		let f = File::open(path)?;
		let mmap = unsafe { Mmap::map(&f)? };

		let (index_offset, index_len) = parse_header(&mmap)?;
		let files = parse_index(&mmap, index_offset, index_len)?;
		let dirs = build_dirs(&files);

		Ok(Self {
			inner: Arc::new(PakInner {
				_mmap_file: f,
				mmap,
				files,
				dirs,
			}),
		})
	}
}

impl AssetReader for PakAssetReader {
	async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
		let key = norm_path(path);
		let Some(e) = self.inner.files.get(key.as_str()) else {
			return Err(AssetReaderError::NotFound(path.to_path_buf()));
		};

		let off = e.offset as usize;
		let end = (e.offset + e.len) as usize;

		if end > self.inner.mmap.len() {
			return Err(AssetReaderError::NotFound(path.to_path_buf()));
		}

		Ok(SliceReader::new(&self.inner.mmap[off..end]))
	}

	async fn read_meta<'a>(
		&'a self,
		path: &'a Path,
	) -> Result<impl bevy::asset::io::Reader + 'a, bevy::asset::io::AssetReaderError> {
		use bevy::asset::io::{AssetReaderError, VecReader};

		Err::<VecReader, _>(AssetReaderError::NotFound(path.to_path_buf()))
	}

	async fn read_directory<'a>(
		&'a self,
		path: &'a Path,
	) -> Result<Box<PathStream>, AssetReaderError> {
		let key = norm_dir(path);
		let Some(list) = self.inner.dirs.get(key.as_str()) else {
			return Ok(Box::new(stream::iter(Vec::<PathBuf>::new())));
		};

		Ok(Box::new(stream::iter(list.clone())))
	}

	async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
		let key = norm_dir(path);
		Ok(self.inner.dirs.contains_key(key.as_str()))
	}
}

fn norm_path(p: &Path) -> String {
	let s = p.to_string_lossy().replace('\\', "/");

	let mut slice: &str = s.as_ref();
	while let Some(rest) = slice.strip_prefix("./") {
		slice = rest;
	}
	while let Some(rest) = slice.strip_prefix('/') {
		slice = rest;
	}

	slice.to_string()
}

fn norm_dir(p: &Path) -> String {
	let mut s = norm_path(p);
	while s.ends_with('/') {
		s.pop();
	}
	s
}

fn parse_header(mmap: &[u8]) -> io::Result<(u64, u64)> {
	if mmap.len() < 24 {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "pak header too small"));
	}

	if mmap[0..4] != MAGIC {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "pak bad magic"));
	}

	let ver = u32::from_le_bytes(mmap[4..8].try_into().unwrap());
	if ver != VERSION {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "pak bad version"));
	}

	let index_offset = u64::from_le_bytes(mmap[8..16].try_into().unwrap());
	let index_len = u64::from_le_bytes(mmap[16..24].try_into().unwrap());

	Ok((index_offset, index_len))
}

fn parse_index(mmap: &[u8], index_offset: u64, index_len: u64) -> io::Result<HashMap<Box<str>, PakEntry>> {
	let off = index_offset as usize;
	let end = (index_offset + index_len) as usize;

	if end > mmap.len() || off > end {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "pak index out of range"));
	}

	let mut cur = off;

	if cur + 4 > end {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "pak index missing count"));
	}

	let count = u32::from_le_bytes(mmap[cur..cur + 4].try_into().unwrap()) as usize;
	cur += 4;

	let mut out = HashMap::with_capacity(count);

	for _ in 0..count {
		if cur + 2 > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "pak index truncated"));
		}

		let plen = u16::from_le_bytes(mmap[cur..cur + 2].try_into().unwrap()) as usize;
		cur += 2;

		if cur + plen > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "pak index bad path"));
		}

		let path = std::str::from_utf8(&mmap[cur..cur + plen])
			.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "pak path utf8"))?;
		cur += plen;

		if cur + 16 > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "pak index missing entry"));
		}

		let offset = u64::from_le_bytes(mmap[cur..cur + 8].try_into().unwrap());
		let len = u64::from_le_bytes(mmap[cur + 8..cur + 16].try_into().unwrap());
		cur += 16;

		out.insert(path.into(), PakEntry { offset, len });
	}

	Ok(out)
}

fn build_dirs(files: &HashMap<Box<str>, PakEntry>) -> HashMap<Box<str>, Vec<PathBuf>> {
	let mut dirs: HashMap<Box<str>, Vec<PathBuf>> = HashMap::new();

	dirs.entry("".into()).or_default();

	for k in files.keys() {
		let pb = PathBuf::from(k.as_ref());

		let mut parent = PathBuf::new();
		for comp in pb.components() {
			parent.push(comp);
			let dir_key = parent.parent().map(norm_dir).unwrap_or_else(|| "".to_string());
			dirs.entry(dir_key.into()).or_default();
		}

		if let Some(p) = pb.parent() {
			let dir_key = norm_dir(p);
			dirs.entry(dir_key.into()).or_default().push(pb.clone());
		} else {
			dirs.entry("".into()).or_default().push(pb.clone());
		}
	}

	dirs
}
