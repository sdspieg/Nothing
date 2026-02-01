use crate::index::FileIndex;
use anyhow::{Context, Result};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

/// Save index to disk using bincode serialization
pub fn save_index(index: &FileIndex, path: &str) -> Result<()> {
    let file = fs::File::create(path)
        .with_context(|| format!("Failed to create index file: {}", path))?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, index)
        .with_context(|| "Failed to serialize index")?;
    Ok(())
}

/// Load index from disk
pub fn load_index(path: &str) -> Result<FileIndex> {
    let file = fs::File::open(path)
        .with_context(|| format!("Failed to open index file: {}", path))?;
    let reader = BufReader::new(file);
    let index = bincode::deserialize_from(reader)
        .with_context(|| "Failed to deserialize index")?;
    Ok(index)
}

/// Get default index path for a drive
pub fn get_index_path(drive: char) -> Result<String> {
    let dir = get_nothing_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory: {}", dir))?;
    Ok(format!("{}\\index_{}.bin", dir, drive))
}

/// Get the .nothing directory path
pub fn get_nothing_dir() -> Result<String> {
    let username = std::env::var("USERNAME")
        .with_context(|| "Failed to get USERNAME environment variable")?;
    Ok(format!("C:\\Users\\{}\\.nothing", username))
}

/// Get path for USN bookmark file
pub fn get_bookmark_path(drive: char) -> Result<PathBuf> {
    let dir = get_nothing_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory: {}", dir))?;
    Ok(PathBuf::from(format!("{}\\bookmark_{}.dat", dir, drive)))
}

/// Save USN bookmark to disk
#[allow(dead_code)]
pub fn save_bookmark(drive: char, usn: u64) -> Result<()> {
    let path = get_bookmark_path(drive)?;
    fs::write(&path, usn.to_le_bytes())
        .with_context(|| format!("Failed to save bookmark for drive {}", drive))?;
    Ok(())
}

/// Load USN bookmark from disk
#[allow(dead_code)]
pub fn load_bookmark(drive: char) -> Result<u64> {
    let path = get_bookmark_path(drive)?;
    let bytes = fs::read(&path)
        .with_context(|| format!("Failed to read bookmark for drive {}", drive))?;
    let array: [u8; 8] = bytes.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid bookmark file size"))?;
    Ok(u64::from_le_bytes(array))
}
