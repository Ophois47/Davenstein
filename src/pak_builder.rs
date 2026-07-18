/*
Davenstein - by David Petnick

Builds Davenstein's DVPK Asset Package From Every Regular File Beneath a
Selected Asset Root Directory

Files are Collected Recursively, Converted to Portable Relative Paths, and
Sorted by Path to Produce Deterministic Package Output Across Platforms

DVPK Package Layout:
	- Fixed-Size Header
	- Uncompressed Raw File Data
	- File Index

Header Layout:
	- 4 Byte DVPK Magic Signature
	- 4 Byte Package Format Version
	- 8 Byte File Index Offset
	- 8 Byte File Index Length

Raw File Data is Written Sequentially Without Compression

File Index Maps Each Normalized Relative Asset Path to its Byte Offset and
Length Within the Raw File Data Section

Header is Initially Written With Placeholder Index Values Because the Final
Index Position is Unknown Until Every Asset Has Been Written

After the File Index is Appended, the Writer Returns to the Beginning of the
Package and Replaces the Placeholder Header With the Final Index Metadata
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

// DVPK Package Identification and Format Version
const MAGIC: [u8; 4] = *b"DVPK";
const VERSION: u32 = 1;

// Fixed DVPK Header Size
// 4 Byte Magic + 4 Byte Version + 8 Byte Index Offset + 8 Byte Index Length
const HEADER_LEN: usize = 4 + 4 + 8 + 8;

// Metadata for Each File Stored in the DVPK Package
#[derive(Clone)]
struct PakEntry {
	// Portable Package-Relative Path Using Forward Slashes
	rel: String,

	// Source File Path on the Local Filesystem
	abs: PathBuf,

	// Starting Byte Offset Within the Raw File Data Section
	offset: u64,

	// Total Number of Raw File Data Bytes
	len: u64,
}

fn main() -> io::Result<()> {
	// Read Source Asset Root and Destination Package Path
	let (root, out) = parse_args();

	// Recursively Collect Every Regular File Beneath Asset Root
	let mut entries = Vec::new();
	collect_files(&root, &root, &mut entries)?;

	// Sort Normalized Paths to Produce Deterministic Package Output
	entries.sort_by(|a, b| a.rel.cmp(&b.rel));

	// Create Destination Directory When Required
	if let Some(parent) = out.parent() {
		fs::create_dir_all(parent)?;
	}

	// Create Package File and Buffer Sequential Writes
	let f = File::create(&out)?;
	let mut w = BufWriter::new(f);

	// Write Placeholder Header Until Final File Index Location is Known
	write_header_placeholder(&mut w)?;

	// First Asset Begins Immediately After Fixed-Size Header
	let mut cursor = HEADER_LEN as u64;

	// Write Raw Asset Data and Record Each File Location
	for e in entries.iter_mut() {
		let mut src = File::open(&e.abs)?;
		let len = io::copy(&mut src, &mut w)?;

		e.offset = cursor;
		e.len = len;
		cursor += len;
	}

	// File Index Begins Immediately After Final Raw Asset Byte
	let index_offset = cursor;

	// Append File Index After Raw Asset Data
	write_index(&mut w, &entries)?;
	w.flush()?;

	// Recover Underlying File for Final Size Calculation and Header Rewrite
	let mut f = match w.into_inner() {
		Ok(f) => f,
		Err(e) => return Err(e.into_error()),
	};

	// Calculate Final Serialized File Index Length
	let file_len = f.metadata()?.len();
	let index_len = file_len - index_offset;

	// Rewrite Header With Final File Index Offset and Length
	f.seek(SeekFrom::Start(0))?;
	write_header(&mut f, index_offset, index_len)?;

	eprintln!("Wrote '{}'", out.display());
	eprintln!("Files: '{}'", entries.len());

	Ok(())
}

// Command Line Options:
// 	--root <path>	Source Asset Root Directory
// 	--out <path>	Destination DVPK Package Path
//
// Defaults:
// 	Root	assets
// 	Output	assets.pak
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

// Recursively Collects Every Regular File Beneath Current Directory
// Original Root Remains Fixed for Package-Relative Path Generation
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PakEntry>) -> io::Result<()> {
	for ent in fs::read_dir(dir)? {
		let ent = ent?;
		let p = ent.path();
		let ty = ent.file_type()?;

		// Continue Recursion Through Child Directories
		if ty.is_dir() {
			collect_files(root, &p, out)?;
			continue;
		}

		// Ignore Symbolic Links and Other Non-Regular Filesystem Entries
		if !ty.is_file() {
			continue;
		}

		let rel = path_rel_slash(root, &p);

		// Offset and Length are Populated When Raw File Data is Written
		out.push(PakEntry {
			rel,
			abs: p,
			offset: 0,
			len: 0,
		});
	}

	Ok(())
}

// Converts Filesystem Path Into Portable Package-Relative Path
// Package Paths Always Use Forward Slashes Across All Platforms
fn path_rel_slash(root: &Path, p: &Path) -> String {
	let rel = p.strip_prefix(root).unwrap_or(p);

	// Normalize Windows Path Separators Into DVPK Path Separators
	let mut s = rel.to_string_lossy().replace('\\', "/");

	// Remove Leading Current Directory Component When Present
	if let Some(stripped) = s.strip_prefix("./") {
		s = stripped.to_string();
	} else if let Some(stripped) = s.strip_prefix(".\\") {
		s = stripped.to_string();
	}

	s
}

// Writes Fixed-Size Header Before Final File Index Metadata is Known
// Zero Values Reserve Space for Index Offset and Index Length
fn write_header_placeholder(w: &mut impl Write) -> io::Result<()> {
	w.write_all(&MAGIC)?;
	w.write_all(&VERSION.to_le_bytes())?;
	w.write_all(&0u64.to_le_bytes())?;
	w.write_all(&0u64.to_le_bytes())?;

	Ok(())
}

// Writes Final DVPK Header Using Little-Endian Numeric Fields
fn write_header(w: &mut impl Write, index_offset: u64, index_len: u64) -> io::Result<()> {
	w.write_all(&MAGIC)?;
	w.write_all(&VERSION.to_le_bytes())?;
	w.write_all(&index_offset.to_le_bytes())?;
	w.write_all(&index_len.to_le_bytes())?;

	Ok(())
}

// DVPK File Index Layout:
// 	4 Bytes		Number of File Entries
//
// Each File Entry:
// 	2 Bytes		Relative Path Length
// 	N Bytes		Relative Path Encoded as UTF-8
// 	8 Bytes		Raw File Data Offset
// 	8 Bytes		Raw File Data Length
fn write_index(w: &mut impl Write, entries: &[PakEntry]) -> io::Result<()> {
	let count = entries.len() as u32;
	w.write_all(&count.to_le_bytes())?;

	for e in entries {
		let bytes = e.rel.as_bytes();

		// Path Length is Limited to Maximum Value Representable by u16
		let plen = u16::try_from(bytes.len()).unwrap_or(u16::MAX);

		w.write_all(&plen.to_le_bytes())?;
		w.write_all(&bytes[..(plen as usize)])?;
		w.write_all(&e.offset.to_le_bytes())?;
		w.write_all(&e.len.to_le_bytes())?;
	}

	Ok(())
}
