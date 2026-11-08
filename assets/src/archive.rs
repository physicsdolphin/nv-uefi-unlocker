//! Single-member extraction from pinned tar archives (.tar.xz / .tar.zst).
//! Ported from generate_v67.py's guarded tarfile usage: exact-name match,
//! duplicate rejection, regular-files only, 128 MiB member cap.

use std::fs::File;
use std::io::Read;
use std::path::Path;

const MEMBER_LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Debug)]
pub enum ArchiveError {
    Io(std::io::Error),
    DuplicateMember(String),
    NotARegularFile(String),
    MemberTooLarge(String),
    MissingMember(String),
}

impl From<std::io::Error> for ArchiveError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::DuplicateMember(n) => write!(f, "duplicate package member: {n}"),
            Self::NotARegularFile(n) => write!(f, "not a regular file in package: {n}"),
            Self::MemberTooLarge(n) => write!(f, "member exceeds limit: {n}"),
            Self::MissingMember(n) => write!(f, "missing package member: {n}"),
        }
    }
}

fn read_member<R: Read>(archive: &mut tar::Archive<R>, name: &str) -> Result<Vec<u8>, ArchiveError> {
    let mut result: Option<Vec<u8>> = None;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        // makeself tar members are stored with a "./" prefix.
        let path: &str = path.strip_prefix("./").unwrap_or(&path);
        if path != name {
            continue;
        }
        if result.is_some() {
            return Err(ArchiveError::DuplicateMember(name.into()));
        }
        let header = entry.header();
        if !header.entry_type().is_file() || header.size()? > MEMBER_LIMIT {
            return Err(ArchiveError::NotARegularFile(name.into()));
        }
        let mut data = Vec::with_capacity(header.size()? as usize);
        entry.read_to_end(&mut data)?;
        result = Some(data);
    }
    result.ok_or_else(|| ArchiveError::MissingMember(name.into()))
}

pub fn extract_member_xz(path: &Path, name: &str) -> Result<Vec<u8>, ArchiveError> {
    let file = File::open(path)?;
    let decoder = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    read_member(&mut archive, name)
}

pub fn extract_member_zst(path: &Path, name: &str) -> Result<Vec<u8>, ArchiveError> {
    let file = File::open(path)?;
    let decoder = zstd::stream::read::Decoder::new(file)?;
    let mut archive = tar::Archive::new(decoder);
    read_member(&mut archive, name)
}

/// Extract one member from the zstd-tar payload embedded in a NVIDIA
/// makeself 1.6.0-nv `.run` installer (first-party firmware transport,
/// issue #212). The text header's `skip=` is a LINE number, not a byte
/// offset: the payload is `tail -n +$skip | zstd -d | tar`, so we advance
/// past skip-1 newlines and require the zstd frame magic there — anything
/// else means the layout is not the pinned installer's and we refuse.
pub fn extract_member_run(path: &Path, name: &str) -> Result<Vec<u8>, ArchiveError> {
    use std::io::{BufRead, BufReader, Seek, SeekFrom};

    let mut head = Vec::new();
    head.resize(0x800, 0);
    {
        let mut file = File::open(path)?;
        std::io::Read::read_exact(&mut file, &mut head)?;
    }
    // "skip=" must match exactly: the header also has skip_decompress=.
    let skip_line: u64 = String::from_utf8_lossy(&head)
        .lines()
        .find_map(|l| l.strip_prefix("skip="))
        .and_then(|v| v.trim().parse().ok())
        .filter(|&n| n >= 2)
        .ok_or_else(|| ArchiveError::MissingMember("makeself skip= header".into()))?;

    let mut reader = BufReader::new(File::open(path)?);
    let mut line = Vec::new();
    for _ in 1..skip_line {
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Err(ArchiveError::MissingMember("makeself payload".into()));
        }
        line.clear();
    }
    let payload_start = reader.stream_position()?;
    let mut magic = [0u8; 4];
    std::io::Read::read_exact(&mut reader, &mut magic)?;
    if magic != [0x28, 0xb5, 0x2f, 0xfd] {
        return Err(ArchiveError::MissingMember(format!(
            "zstd frame at makeself skip line {skip_line} (offset {payload_start})"
        )));
    }
    reader.seek(SeekFrom::Start(payload_start))?;
    let decoder = zstd::stream::read::Decoder::new(reader)?;
    let mut archive = tar::Archive::new(decoder);
    read_member(&mut archive, name)
}
