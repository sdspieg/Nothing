// Sector-aligned wrapper for raw volume reads
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Result};

const SECTOR_SIZE: usize = 512;

/// A wrapper around File that ensures all reads are sector-aligned
pub struct SectorAlignedReader {
    file: File,
    position: u64,
    buffer: Vec<u8>,
    buffer_start: u64,
    buffer_valid: usize,
}

impl SectorAlignedReader {
    pub fn new(file: File) -> Self {
        Self {
            file,
            position: 0,
            buffer: vec![0u8; SECTOR_SIZE * 16], // 8KB buffer
            buffer_start: 0,
            buffer_valid: 0,
        }
    }

    fn fill_buffer(&mut self) -> Result<()> {
        // Align to sector boundary
        let aligned_pos = (self.position / SECTOR_SIZE as u64) * SECTOR_SIZE as u64;

        // Seek to aligned position
        self.file.seek(SeekFrom::Start(aligned_pos))?;

        // Read full sectors
        let bytes_read = self.file.read(&mut self.buffer)?;

        self.buffer_start = aligned_pos;
        self.buffer_valid = bytes_read;

        Ok(())
    }
}

impl Read for SectorAlignedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Check if we need to fill/refill buffer
        if self.position < self.buffer_start
            || self.position >= self.buffer_start + self.buffer_valid as u64
            || self.buffer_valid == 0
        {
            self.fill_buffer()?;
        }

        // Calculate offset within buffer
        let offset_in_buffer = (self.position - self.buffer_start) as usize;
        let available = self.buffer_valid.saturating_sub(offset_in_buffer);

        if available == 0 {
            return Ok(0); // EOF
        }

        // Copy from buffer
        let to_copy = available.min(buf.len());
        buf[..to_copy].copy_from_slice(
            &self.buffer[offset_in_buffer..offset_in_buffer + to_copy]
        );

        self.position += to_copy as u64;
        Ok(to_copy)
    }
}

impl Seek for SectorAlignedReader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n,
            SeekFrom::Current(n) => {
                if n >= 0 {
                    self.position + n as u64
                } else {
                    self.position.saturating_sub((-n) as u64)
                }
            }
            SeekFrom::End(n) => {
                let file_size = self.file.metadata()?.len();
                if n >= 0 {
                    file_size + n as u64
                } else {
                    file_size.saturating_sub((-n) as u64)
                }
            }
        };

        self.position = new_pos;
        Ok(new_pos)
    }
}
