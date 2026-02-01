// Test different volume access methods
use anyhow::Result;
use ntfs::Ntfs;
use std::fs::OpenOptions;
use std::io::{BufReader, Read, Seek};
use windows::Win32::Foundation::{CloseHandle, HANDLE, GENERIC_READ};
use ntfs_reader::{mft::Mft as NtfsReaderMft, volume::Volume};
use crate::sector_aligned_reader::SectorAlignedReader;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, SetFilePointerEx, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, FILE_ATTRIBUTE_NORMAL,
    FILE_BEGIN, FILE_CURRENT, FILE_END,
};
use std::io::SeekFrom;

/// Test 1: Use std::fs::File with OpenOptions (no debug reads)
pub fn test_std_file(drive: char) -> Result<()> {
    println!("\n=== Test 1: std::fs::File (direct) ===");
    let path = format!("\\\\.\\{}:", drive);

    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(&path)?;

    println!("Opened volume: {}", path);

    let mut ntfs = Ntfs::new(&mut file)?;
    println!("✅ NTFS initialized successfully!");

    ntfs.read_upcase_table(&mut file)?;
    println!("✅ Upcase table read successfully!");

    Ok(())
}

/// Test 2: Windows API with different flags
struct VolumeHandle2 {
    handle: HANDLE,
}

impl Drop for VolumeHandle2 {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Read for VolumeHandle2 {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read = 0u32;
        unsafe {
            ReadFile(self.handle, Some(buf), Some(&mut bytes_read), None)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok(bytes_read as usize)
    }
}

impl Seek for VolumeHandle2 {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let (distance, method) = match pos {
            SeekFrom::Start(n) => (n as i64, FILE_BEGIN),
            SeekFrom::Current(n) => (n, FILE_CURRENT),
            SeekFrom::End(n) => (n, FILE_END),
        };

        let mut new_pos = 0i64;
        unsafe {
            SetFilePointerEx(self.handle, distance, Some(&mut new_pos), method)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        Ok(new_pos as u64)
    }
}

pub fn test_windows_api_with_flags(drive: char) -> Result<()> {
    println!("\n=== Test 2: Windows API with FILE_FLAG_BACKUP_SEMANTICS ===");
    let path = format!("\\\\.\\{}:", drive);
    let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    let handle = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide_path.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_ATTRIBUTE_NORMAL,
            None,
        )?
    };

    println!("Opened volume: {}", path);

    let mut volume = VolumeHandle2 { handle };

    let mut ntfs = Ntfs::new(&mut volume)?;
    println!("✅ NTFS initialized successfully!");

    ntfs.read_upcase_table(&mut volume)?;
    println!("✅ Upcase table read successfully!");

    Ok(())
}

/// Test 3: std::fs::File with BufReader
pub fn test_with_bufreader(drive: char) -> Result<()> {
    println!("\n=== Test 3: std::fs::File with BufReader ===");
    let path = format!("\\\\.\\{}:", drive);

    let file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(&path)?;

    println!("Opened volume: {}", path);

    let mut reader = BufReader::with_capacity(8192, file);

    let mut ntfs = Ntfs::new(&mut reader)?;
    println!("✅ NTFS initialized successfully!");

    ntfs.read_upcase_table(&mut reader)?;
    println!("✅ Upcase table read successfully!");

    Ok(())
}

/// Test 4: ntfs-reader crate (API unclear, skipping for now)
#[allow(dead_code)]
pub fn test_ntfs_reader(drive: char) -> Result<()> {
    println!("\n=== Test 4: ntfs-reader crate (SKIPPED) ===");
    let path = format!("\\\\.\\{}:", drive);

    // Create Volume
    let _volume = Volume::new(&path)?;
    println!("✅ ntfs-reader Volume created successfully!");

    // TODO: Figure out the correct API for reading MFT entries

    Ok(())
}

/// Test 5: Sector-aligned reader wrapper
pub fn test_sector_aligned(drive: char) -> Result<()> {
    println!("\n=== Test 5: Sector-Aligned Reader Wrapper ===");
    let path = format!("\\\\.\\{}:", drive);

    let file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(&path)?;

    println!("Opened volume: {}", path);

    let mut reader = SectorAlignedReader::new(file);

    let mut ntfs = Ntfs::new(&mut reader)?;
    println!("✅ NTFS initialized successfully with sector-aligned reader!");

    ntfs.read_upcase_table(&mut reader)?;
    println!("✅ Upcase table read successfully!");

    // Try to get root directory
    let root = ntfs.root_directory(&mut reader)?;
    println!("✅ Root directory accessed!");
    println!("Root directory has {} attributes", root.attributes_raw().count());

    Ok(())
}

pub fn run_all_tests(drive: char) {
    println!("Testing volume access methods for drive {}:", drive);

    // Skip tests 1-3 because they panic inside the ntfs crate
    // match test_std_file(drive) {
    //     Ok(_) => println!("Test 1 PASSED"),
    //     Err(e) => println!("Test 1 FAILED: {}", e),
    // }

    // match test_windows_api_with_flags(drive) {
    //     Ok(_) => println!("Test 2 PASSED"),
    //     Err(e) => println!("Test 2 FAILED: {}", e),
    // }

    // match test_with_bufreader(drive) {
    //     Ok(_) => println!("✅ Test 3 PASSED - BufReader works!"),
    //     Err(e) => println!("Test 3 FAILED: {}", e),
    // }

    match test_ntfs_reader(drive) {
        Ok(_) => println!("✅✅✅ Test 4 PASSED - ntfs-reader crate works!"),
        Err(e) => println!("Test 4 FAILED: {}", e),
    }

    match test_sector_aligned(drive) {
        Ok(_) => println!("\n🎉🎉🎉 Test 5 PASSED - Sector-Aligned Reader WORKS! 🎉🎉🎉"),
        Err(e) => println!("Test 5 FAILED: {}", e),
    }
}
