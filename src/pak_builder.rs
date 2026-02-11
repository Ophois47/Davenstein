/*
Davenstein - by David Petnick
*/
use std::fs::{self, File};
use std::io::{
	self,
	BufWriter,
	Seek,
	SeekFrom,
	Write,
};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 4] = *b"DVPK";
const VERSION: u32 = 1;

#[derive(Clone)]
struct PakEntry {
	rel: String,
	abs: PathBuf,
	offset: u64,
	len: u64,
}

fn main() -> io::Result<()> {
	let (root, out) = parse_args();
	let mut entries = Vec::new();
	collect_files(&root, &root, &mut entries)?;

	entries.sort_by(|a, b| a.rel.cmp(&b.rel));

	if let Some(parent) = out.parent() {
		fs::create_dir_all(parent)?;
	}

	let f = File::create(&out)?;
	let mut w = BufWriter::new(f);

	write_header_placeholder(&mut w)?;

	let mut cursor = HEADER_LEN as u64;
	for e in entries.iter_mut() {
		let mut src = File::open(&e.abs)?;
		let len = io::copy(&mut src, &mut w)?;
		e.offset = cursor;
		e.len = len;
		cursor += len;
	}

	let index_offset = cursor;
	write_index(&mut w, &entries)?;
	w.flush()?;

	let mut f = match w.into_inner() {
		Ok(f) => f,
		Err(e) => return Err(e.into_error()),
	};

	let file_len = f.metadata()?.len();
	let index_len = file_len - index_offset;

	f.seek(SeekFrom::Start(0))?;
	write_header(&mut f, index_offset, index_len)?;

	eprintln!("wrote {}", out.display());
	eprintln!("files {}", entries.len());
	Ok(())
}

fn parse_args() -> (PathBuf, PathBuf) {
	let mut root = PathBuf::from("assets");
	let mut out = PathBuf::from("assets.pak");

	let mut it = std::env::args().skip(1);
	while let Some(a) = it.next() {
		match a.as_str() {
			"--root" => {
				if let Some(v) = it.next() {
					root = PathBuf::from(v);
				}
			}
			"--out" => {
				if let Some(v) = it.next() {
					out = PathBuf::from(v);
				}
			}
			_ => {}
		}
	}

	(root, out)
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PakEntry>) -> io::Result<()> {
	for ent in fs::read_dir(dir)? {
		let ent = ent?;
		let p = ent.path();
		let ty = ent.file_type()?;

		if ty.is_dir() {
			collect_files(root, &p, out)?;
			continue;
		}

		if !ty.is_file() {
			continue;
		}

		let rel = path_rel_slash(root, &p);
		out.push(PakEntry {
			rel,
			abs: p,
			offset: 0,
			len: 0,
		});
	}

	Ok(())
}

fn path_rel_slash(root: &Path, p: &Path) -> String {
	let rel = p.strip_prefix(root).unwrap_or(p);

	let mut s = rel.to_string_lossy().replace('\\', "/");

	if let Some(stripped) = s.strip_prefix("./") {
		s = stripped.to_string();
	} else if let Some(stripped) = s.strip_prefix(".\\") {
		s = stripped.to_string();
	}

	s
}

const HEADER_LEN: usize = 4 + 4 + 8 + 8;

fn write_header_placeholder(w: &mut impl Write) -> io::Result<()> {
	w.write_all(&MAGIC)?;
	w.write_all(&VERSION.to_le_bytes())?;
	w.write_all(&0u64.to_le_bytes())?;
	w.write_all(&0u64.to_le_bytes())?;
	Ok(())
}

fn write_header(w: &mut impl Write, index_offset: u64, index_len: u64) -> io::Result<()> {
	w.write_all(&MAGIC)?;
	w.write_all(&VERSION.to_le_bytes())?;
	w.write_all(&index_offset.to_le_bytes())?;
	w.write_all(&index_len.to_le_bytes())?;
	Ok(())
}

fn write_index(w: &mut impl Write, entries: &[PakEntry]) -> io::Result<()> {
	let count = entries.len() as u32;
	w.write_all(&count.to_le_bytes())?;

	for e in entries {
		let bytes = e.rel.as_bytes();
		let plen = u16::try_from(bytes.len()).unwrap_or(u16::MAX);

		w.write_all(&plen.to_le_bytes())?;
		w.write_all(&bytes[..(plen as usize)])?;
		w.write_all(&e.offset.to_le_bytes())?;
		w.write_all(&e.len.to_le_bytes())?;
	}

	Ok(())
}
