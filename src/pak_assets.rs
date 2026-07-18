/*
Davenstein - by David Petnick

Provides Bevy With a Custom Asset Reader That Loads Davenstein Assets Directly
From the DVPK Asset Package

Release Builds Automatically Use assets.pak While Debug Builds Continue Using
the Normal Filesystem Asset Source Unless DAVENSTEIN_USE_PAK is Set

DAVENSTEIN_PAK_PATH May be Used to Override the Default Package Location

Default Package Location:
	- assets.pak Beside the Running Davenstein Executable

DVPK Package Layout:
	- Fixed-Size Header
	- Uncompressed Raw File Data
	- File Index

Header Layout:
	- 4 Byte DVPK Magic Signature
	- 4 Byte Package Format Version
	- 8 Byte File Index Offset
	- 8 Byte File Index Length

File Index Layout:
	- 4 Byte File Entry Count

Each File Entry Contains:
	- 2 Byte Relative Path Length
	- UTF-8 Relative Path
	- 8 Byte Raw File Data Offset
	- 8 Byte Raw File Data Length

Package File is Memory Mapped so Asset Reads Can Return Slices of Existing
Package Data Without Reopening or Copying Individual Asset Files

File Index is Parsed Into a Path Lookup Table for Direct Asset Access

A Separate Directory Lookup Table is Generated From Indexed File Paths so
Bevy Can Enumerate Package Directories Through its AssetReader Interface

All Asset Paths are Normalized to Forward Slashes Without Leading Current
Directory or Root Separators to Match Paths Written by pak_builder.rs
*/

use bevy::{
	asset::{
		io::{
			AssetReader,
			AssetReaderError,
			PathStream,
			Reader,
			SliceReader,
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

// DVPK Package Identification and Supported Format Version
const MAGIC: [u8; 4] = *b"DVPK";
const VERSION: u32 = 1;

// Bevy Plugin Responsible for Replacing Default Asset Source With DVPK Reader
pub struct PakAssetsPlugin;

impl Plugin for PakAssetsPlugin {
	fn build(&self, app: &mut App) {
		use bevy::asset::AssetApp;
		use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};

		// Keep Normal Filesystem Assets in Debug Builds Unless Explicitly Enabled
		if !should_use_pak() {
			return;
		}

		// Resolve Environment Override or Default Package Path Beside Executable
		let Some(pak_path) = resolve_pak_path() else {
			warn!("Pak enabled but no pak path resolved");
			return;
		};

		// Leave Existing Bevy Asset Source Unchanged When Package is Missing
		if !pak_path.exists() {
			warn!("Your assets.pak was not found at {}", pak_path.display());
			return;
		}

		info!("Using assets.pak '{}'", pak_path.display());

		// Open Package Once and Share Parsed Memory-Mapped State Between Readers
		let inner = match PakAssetReader::open(pak_path.as_path()) {
			Ok(r) => r.inner,
			Err(err) => {
				warn!("Failed to open pak {}: {}", pak_path.display(), err);
				return;
			}
		};

		// Replace Bevy Default Asset Source With DVPK-Backed Asset Reader
		app.register_asset_source(
			AssetSourceId::Default,
			AssetSourceBuilder::new(move || Box::new(PakAssetReader { inner: inner.clone() })),
		);
	}
}

// Resolves Explicit Package Path Before Falling Back to Executable Directory
fn resolve_pak_path() -> Option<PathBuf> {
	if let Some(p) = std::env::var_os("DAVENSTEIN_PAK_PATH") {
		return Some(PathBuf::from(p));
	}

	default_pak_path()
}

// Package Selection Policy:
// 	Debug Builds	Use Filesystem Assets Unless DAVENSTEIN_USE_PAK is Set
// 	Release Builds	Always Use DVPK Asset Package
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

// Default Package Path is assets.pak Beside Running Executable
fn default_pak_path() -> Option<PathBuf> {
	let exe = std::env::current_exe().ok()?;
	let dir = exe.parent()?;

	Some(dir.join("assets.pak"))
}

// Location of One Asset Within Memory-Mapped Raw File Data
#[derive(Clone)]
struct PakEntry {
	// Starting Byte Offset Within Complete DVPK Package
	offset: u64,

	// Total Number of Raw Asset Bytes
	len: u64,
}

// Shared Parsed State for Open DVPK Package
struct PakInner {
	// Keep Package File Open for Lifetime of Memory Mapping
	_mmap_file: File,

	// Read-Only Memory Mapping Covering Complete DVPK Package
	mmap: Mmap,

	// Normalized Asset Path to Raw Data Location
	files: HashMap<Box<str>, PakEntry>,

	// Normalized Directory Path to Immediate Indexed File Paths
	dirs: HashMap<Box<str>, Vec<PathBuf>>,
}

// Bevy AssetReader Backed by Shared DVPK Package State
struct PakAssetReader {
	inner: Arc<PakInner>,
}

impl PakAssetReader {
	// Opens Package, Maps Contents, Validates Header, Parses Index, and Builds Directories
	fn open(path: &Path) -> io::Result<Self> {
		let f = File::open(path)?;

		// Mapping is Safe While File Remains Open and Mapping is Only Read
		let mmap = unsafe { Mmap::map(&f)? };

		// Validate Package Header and Locate Serialized File Index
		let (index_offset, index_len) = parse_header(&mmap)?;

		// Build Direct File Lookup From Serialized DVPK Index
		let files = parse_index(&mmap, index_offset, index_len)?;

		// Derive Directory Listings Required by Bevy AssetReader
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
	// Returns Read-Only Slice of Memory-Mapped Asset Data
	async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
		let key = norm_path(path);

		// Asset Paths Must Match Normalized Paths Stored in DVPK File Index
		let Some(e) = self.inner.files.get(key.as_str()) else {
			return Err(AssetReaderError::NotFound(path.to_path_buf()));
		};

		let off = e.offset as usize;
		let end = (e.offset + e.len) as usize;

		// Reject Indexed Data Range Extending Beyond Memory-Mapped Package
		if end > self.inner.mmap.len() {
			return Err(AssetReaderError::NotFound(path.to_path_buf()));
		}

		Ok(SliceReader::new(&self.inner.mmap[off..end]))
	}

	// DVPK Packages Do Not Store Separate Bevy Metadata Files
	async fn read_meta<'a>(
		&'a self,
		path: &'a Path,
	) -> Result<impl bevy::asset::io::Reader + 'a, bevy::asset::io::AssetReaderError> {
		use bevy::asset::io::{AssetReaderError, VecReader};

		Err::<VecReader, _>(AssetReaderError::NotFound(path.to_path_buf()))
	}

	// Returns Indexed Files Associated With Requested Package Directory
	async fn read_directory<'a>(
		&'a self,
		path: &'a Path,
	) -> Result<Box<PathStream>, AssetReaderError> {
		let key = norm_dir(path);

		// Unknown Directories Produce Empty Stream Rather Than Reader Error
		let Some(list) = self.inner.dirs.get(key.as_str()) else {
			return Ok(Box::new(stream::iter(Vec::<PathBuf>::new())));
		};

		Ok(Box::new(stream::iter(list.clone())))
	}

	// Reports Whether Normalized Path Exists in Generated Directory Table
	async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
		let key = norm_dir(path);

		Ok(self.inner.dirs.contains_key(key.as_str()))
	}
}

// Converts Bevy Asset Path Into Normalized DVPK Index Path
// Package Paths Use Forward Slashes Without Leading ./ or / Components
fn norm_path(p: &Path) -> String {
	let s = p.to_string_lossy().replace('\\', "/");

	let mut slice: &str = s.as_ref();

	// Remove Any Repeated Current Directory Prefixes
	while let Some(rest) = slice.strip_prefix("./") {
		slice = rest;
	}

	// Remove Any Repeated Leading Root Separators
	while let Some(rest) = slice.strip_prefix('/') {
		slice = rest;
	}

	slice.to_string()
}

// Normalizes Directory Path and Removes Trailing Separators
fn norm_dir(p: &Path) -> String {
	let mut s = norm_path(p);

	while s.ends_with('/') {
		s.pop();
	}

	s
}

// Validates Fixed-Size DVPK Header and Returns File Index Location
//
// Header Layout:
// 	4 Bytes		DVPK Magic Signature
// 	4 Bytes		Package Format Version
// 	8 Bytes		File Index Offset
// 	8 Bytes		File Index Length
fn parse_header(mmap: &[u8]) -> io::Result<(u64, u64)> {
	// Complete DVPK Header Requires Exactly 24 Available Bytes
	if mmap.len() < 24 {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak header too small"));
	}

	// Reject Files Without DVPK Package Signature
	if mmap[0..4] != MAGIC {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak bad magic"));
	}

	// Reject Packages Written With Unsupported Format Version
	let ver = u32::from_le_bytes(mmap[4..8].try_into().unwrap());
	if ver != VERSION {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak bad version"));
	}

	// Read Little-Endian File Index Location From Header
	let index_offset = u64::from_le_bytes(mmap[8..16].try_into().unwrap());
	let index_len = u64::from_le_bytes(mmap[16..24].try_into().unwrap());

	Ok((index_offset, index_len))
}

// Parses Serialized DVPK File Index Into Normalized Path Lookup Table
//
// File Index Layout:
// 	4 Bytes		Number of File Entries
//
// Each File Entry:
// 	2 Bytes		Relative Path Length
// 	N Bytes		Relative Path Encoded as UTF-8
// 	8 Bytes		Raw File Data Offset
// 	8 Bytes		Raw File Data Length
fn parse_index(
	mmap: &[u8],
	index_offset: u64,
	index_len: u64,
) -> io::Result<HashMap<Box<str>, PakEntry>> {
	let off = index_offset as usize;
	let end = (index_offset + index_len) as usize;

	// Reject File Index Range Outside Memory-Mapped Package
	if end > mmap.len() || off > end {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak index out of range"));
	}

	let mut cur = off;

	// File Index Must Begin With 4 Byte Entry Count
	if cur + 4 > end {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak index missing count"));
	}

	let count = u32::from_le_bytes(mmap[cur..cur + 4].try_into().unwrap()) as usize;
	cur += 4;

	// Preallocate Lookup Table Using Declared File Entry Count
	let mut out = HashMap::with_capacity(count);

	for _ in 0..count {
		// Each Entry Must Begin With 2 Byte Path Length
		if cur + 2 > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak index truncated"));
		}

		let plen = u16::from_le_bytes(mmap[cur..cur + 2].try_into().unwrap()) as usize;
		cur += 2;

		// Declared Path Must Remain Within File Index Bounds
		if cur + plen > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak index bad path"));
		}

		// Package Paths Must Contain Valid UTF-8
		let path = std::str::from_utf8(&mmap[cur..cur + plen])
			.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Pak path utf8"))?;
		cur += plen;

		// Each Entry Requires 8 Byte Offset and 8 Byte Length
		if cur + 16 > end {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "Pak index missing entry"));
		}

		let offset = u64::from_le_bytes(mmap[cur..cur + 8].try_into().unwrap());
		let len = u64::from_le_bytes(mmap[cur + 8..cur + 16].try_into().unwrap());
		cur += 16;

		out.insert(path.into(), PakEntry { offset, len });
	}

	Ok(out)
}

// Builds Directory Lookup Required by Bevy From Indexed File Paths
// Empty String Represents Root of DVPK Asset Tree
fn build_dirs(files: &HashMap<Box<str>, PakEntry>) -> HashMap<Box<str>, Vec<PathBuf>> {
	let mut dirs: HashMap<Box<str>, Vec<PathBuf>> = HashMap::new();

	// Root Directory Always Exists Even When Package Contains No Files
	dirs.entry("".into()).or_default();

	for k in files.keys() {
		let pb = PathBuf::from(k.as_ref());

		// Register Every Parent Directory Component Found in File Path
		let mut parent = PathBuf::new();
		for comp in pb.components() {
			parent.push(comp);

			let dir_key = parent
				.parent()
				.map(norm_dir)
				.unwrap_or_else(|| "".to_string());

			dirs.entry(dir_key.into()).or_default();
		}

		// Add Complete File Path to its Immediate Parent Directory Listing
		if let Some(p) = pb.parent() {
			let dir_key = norm_dir(p);

			dirs.entry(dir_key.into()).or_default().push(pb.clone());
		} else {
			dirs.entry("".into()).or_default().push(pb.clone());
		}
	}

	dirs
}
