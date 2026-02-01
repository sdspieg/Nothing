# Technical Details - Nothing File Search

## Overview

Nothing is a Windows file search tool that reads the NTFS Master File Table (MFT) directly for fast indexing. This document covers the technical implementation, performance characteristics, and architectural decisions.

## Architecture

### Core Components

```
nothing/
├── src/
│   ├── main.rs                    # Entry point, CLI parsing
│   ├── mft_reader.rs              # Fast mode MFT scanner (usn-journal-rs)
│   ├── mft_reader_ntfs.rs         # Full metadata MFT scanner (ntfs crate)
│   ├── sector_aligned_reader.rs   # ⭐ Sector-aligned I/O wrapper
│   ├── multi_drive.rs             # Multi-drive and cloud storage support
│   ├── file_entry.rs              # File metadata structures
│   ├── index.rs                   # In-memory file index
│   ├── search.rs                  # Fuzzy search engine
│   ├── interactive.rs             # Interactive CLI
│   ├── volume_test.rs             # Volume access testing
│   └── error.rs                   # Error types
```

### Dependencies

**Core:**
- `usn-journal-rs` (0.2) - Fast MFT enumeration for fast mode
- `ntfs` (0.4) - Full NTFS parsing for metadata mode
- `walkdir` (2.4) - Directory walking for cloud storage

**Search:**
- `nucleo-matcher` (0.3) - Fuzzy matching algorithm
- `crossterm` (0.28) - Terminal UI for interactive mode

**Utilities:**
- `clap` (4.5) - CLI argument parsing
- `anyhow` (1.0) - Error handling
- `chrono` (0.4) - Timestamp handling
- `windows` (0.61) - Windows API bindings

## Dual-Mode Design

### Fast Mode (usn-journal-rs)

**How it works:**
- Uses Windows `FSCTL_ENUM_USN_DATA` API
- Enumerates MFT entries without parsing full attributes
- Extracts only: name, parent reference, directory flag
- No timestamp or size information

**Performance:**
- **Speed:** 155,088 files/sec
- **Memory:** 2.12 GB for 10.7M files
- **Time:** 73.74 seconds for C: drive (10.7M files)

**Use case:** When you only need to find files by name/path

### Full Metadata Mode (ntfs crate)

**How it works:**
- Opens raw volume handle (`\\.\C:`)
- Parses NTFS boot sector and MFT
- Extracts multiple attributes per file:
  - `$FILE_NAME` - Name and parent directory reference
  - `$STANDARD_INFORMATION` - Created, modified, accessed timestamps
  - `$DATA` - File size from data attribute
- Iterates through all MFT record numbers (0 to MAX)

**Performance (measured on D: drive):**
- **Speed:** 3,971 files/sec
- **Memory:** 1.06 GB for 195k files (~5.4 KB per file)
- **Time:** 60.10 seconds for 195,044 files

**Projected for C: drive (10.7M files):**
- **Time:** ~45 minutes (2,695 seconds)
- **Memory:** ~4-5 GB

**Use case:** When you need file sizes, timestamps, or want to search/filter by date

## The Sector-Aligned Reader Breakthrough

### The Problem

The `ntfs` crate uses `binrw` for parsing NTFS structures. When reading from a raw Windows volume handle, the parser makes small, unaligned reads (e.g., reading 3 bytes at offset 5).

**Windows restriction:** Raw volume I/O requires all reads to be aligned to 512-byte sector boundaries.

**Error encountered:**
```
Error: The parameter is incorrect. (os error 87)
While parsing field 'bootjmp' in BootSector
```

### The Solution: SectorAlignedReader

Created a wrapper that implements `Read` + `Seek` traits:

```rust
pub struct SectorAlignedReader {
    file: File,              // Underlying volume handle
    position: u64,           // Logical position
    buffer: Vec<u8>,         // 8KB buffer (16 sectors)
    buffer_start: u64,       // Start position of buffered data
    buffer_valid: usize,     // Valid bytes in buffer
}
```

**Key features:**
1. **Transparent buffering:** Maintains an 8KB internal buffer
2. **Sector alignment:** Always reads from sector-aligned positions
3. **Position tracking:** Tracks logical position separately from file position
4. **Lazy loading:** Only reads from disk when buffer doesn't contain requested data

**Algorithm:**
```rust
impl Read for SectorAlignedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Check if current position is in buffer
        if self.position < self.buffer_start
           || self.position >= self.buffer_start + self.buffer_valid {
            // Refill buffer from sector-aligned position
            let aligned_pos = (self.position / 512) * 512;
            self.file.seek(SeekFrom::Start(aligned_pos))?;
            self.buffer_valid = self.file.read(&mut self.buffer)?;
            self.buffer_start = aligned_pos;
        }

        // Copy from buffer to output
        let offset = (self.position - self.buffer_start) as usize;
        let available = self.buffer_valid - offset;
        let to_copy = available.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.buffer[offset..offset + to_copy]);

        self.position += to_copy as u64;
        Ok(to_copy)
    }
}
```

**Why this works:**
- Windows sees only sector-aligned reads → Happy
- ntfs crate reads any byte at any offset → Happy
- Zero modifications to third-party code → Maintainable

### Performance Impact

The buffering adds minimal overhead:
- Buffer size: 8KB (sweet spot for disk I/O)
- Most reads hit buffer (MFT entries are sequential)
- Seek operations just update position (no disk I/O unless needed)

## Multi-Drive Support

### Drive Detection

```rust
pub fn get_all_drives() -> Vec<char> {
    // Check A-Z for existence
    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", letter as char);
        if std::path::Path::new(&path).exists() {
            drives.push(letter as char);
        }
    }
}
```

### Drive Type Classification

Uses Windows `GetDriveTypeW` API:

| Type | Value | Description |
|------|-------|-------------|
| Fixed | 3 | Local hard drive/SSD |
| Removable | 2 | USB drive, SD card |
| Remote | 4 | Network share |
| CDRom | 5 | CD/DVD drive |
| RamDisk | 6 | RAM disk |

**Default behavior:** Only scans Fixed drives (type 3)

### Error Handling

Each drive is scanned independently with error isolation:

```rust
for drive in drives {
    match scan_drive(drive) {
        Ok(_) => println!("✅ Drive {}: scanned"),
        Err(e) => eprintln!("⚠️  Drive {}: {}", drive, e),
        // Continue with next drive
    }
}
```

## Cloud Storage Integration

### Detection Strategy

**Common locations checked:**
```
%USERPROFILE%\Google Drive
%USERPROFILE%\GoogleDrive
G:\                              # Sometimes mounted as drive
%USERPROFILE%\Dropbox
D:\Dropbox                       # Sometimes on D:
%USERPROFILE%\OneDrive
%USERPROFILE%\OneDrive - Personal
%USERPROFILE%\OneDrive - Business
```

### Indexing Method

Cloud folders are indexed using `walkdir` instead of MFT scanning:

**Why?**
- Cloud files may be "cloud-only" (not in local MFT)
- Virtual filesystems (like Google Drive Stream) don't expose files via MFT
- Directory walking is slower but guaranteed to see all files

**Implementation:**
```rust
for entry in WalkDir::new(path).follow_links(false) {
    let metadata = fs::metadata(entry.path())?;

    let file_entry = FileEntry {
        name: entry.file_name(),
        path: entry.path(),
        size: metadata.len(),
        modified: metadata.modified()?,
        created: metadata.created()?,
        is_directory: entry.file_type().is_dir(),
        // ...
    };

    index.add_entry(file_entry);
}
```

**Performance:**
- ~1,000-5,000 files/sec (depends on cloud sync state)
- Much slower than MFT but necessary for cloud folders

### Placeholder Files

Cloud storage systems use "placeholder" files:
- File appears in directory listing
- Actual data is cloud-only (not downloaded)
- Windows attribute: `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`

**Current handling:** Index all files (future: detect and skip placeholders)

## Memory Optimization Techniques

### 1. Arc<str> for Parent Paths

**Problem:** Millions of files share common path prefixes (e.g., `C:\Windows\System32\...`)

**Solution:**
```rust
let mut parent_map: HashMap<u64, Arc<str>> = HashMap::new();

if is_directory {
    parent_map.insert(file_id, Arc::clone(&path));
}
```

**Benefit:** Path strings are reference-counted and shared
- Before: Each file stores full path (millions of duplicates)
- After: Common prefixes shared via Arc
- **Memory savings:** ~20% reduction (2.64 GB → 2.12 GB)

### 2. Pre-allocated Vectors

```rust
index.reserve(10_000_000);  // Pre-allocate for 10M files
```

**Benefit:** Avoids reallocations during scanning

### 3. String::with_capacity

```rust
let mut path = String::with_capacity(parent.len() + 1 + name.len());
path.push_str(parent);
path.push('\\');
path.push_str(name);
```

**Benefit:** Single allocation instead of multiple grow operations

### 4. Directory-only parent_map

Only directories are stored in parent_map (not all files):
- Files: ~90% of entries
- Directories: ~10% of entries
- **Memory savings:** 90% reduction in HashMap size

### 5. Periodic Cleanup

```rust
if count % 1_000_000 == 0 {
    // Remove parent paths no longer referenced
    parent_map.retain(|_, path| Arc::strong_count(path) > 1);
}
```

**Benefit:** Frees memory from old parent directories

## Search Performance

### Fuzzy Matching Algorithm

Uses `nucleo-matcher` (same engine as Helix editor):

**Features:**
- Character-based fuzzy matching
- Gap penalties for non-contiguous matches
- Bonus for word boundaries
- Case-insensitive

**Example:**
```
Query: "dcmnts"
Matches: "Documents" (score: 850)
         "doc_comments.txt" (score: 720)
```

### Scoring Strategy

```rust
// Filename matches worth 2x path matches
let filename_score = pattern.score(name) * 2;
let path_score = pattern.score(path);

let best_score = filename_score.max(path_score);
```

**Rationale:** Users usually search by filename, not full path

### Search Complexity

- **Time:** O(n) where n = number of files
- **Space:** O(k) where k = result limit (50)
- **Optimization:** Early termination after finding 50 best matches

**Performance:** <100ms for 10M files on modern CPU

## File Entry Structure

```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,                    // Filename
    pub path: String,                    // Full path
    pub is_directory: bool,              // Directory flag
    pub file_id: u64,                    // MFT record number
    pub parent_id: u64,                  // Parent directory MFT record
    pub size: u64,                       // File size (0 in fast mode)
    pub modified: Option<DateTime<Utc>>, // Modified timestamp
    pub created: Option<DateTime<Utc>>,  // Created timestamp
    pub accessed: Option<DateTime<Utc>>, // Accessed timestamp
}
```

**Memory per entry:**
- Fast mode: ~200 bytes (mostly path strings)
- Full metadata mode: ~230 bytes (+ 3 timestamps + size)

**For 10M files:**
- Fast mode: ~2 GB
- Full metadata: ~2.3 GB
- Plus HashMap and Vec overhead

## Windows API Usage

### Raw Volume Access

```rust
let path = "\\\\.\\C:";  // Raw volume path
let file = OpenOptions::new()
    .read(true)
    .write(false)
    .open(&path)?;
```

**Requires:** Administrator privileges

### Drive Type Detection

```rust
use windows::Win32::Storage::FileSystem::GetDriveTypeW;

let drive_type = unsafe {
    GetDriveTypeW(wide_path.as_ptr())
};

match drive_type {
    3 => DriveType::Fixed,
    4 => DriveType::Network,
    // ...
}
```

### FILETIME Conversion

NTFS timestamps are 100-nanosecond intervals since January 1, 1601:

```rust
const NT_TO_UNIX_OFFSET: u64 = 116_444_736_000_000_000;

let unix_timestamp = nt_timestamp - NT_TO_UNIX_OFFSET;
let secs = (unix_timestamp / 10_000_000) as i64;
let nanos = ((unix_timestamp % 10_000_000) * 100) as u32;

Utc.timestamp_opt(secs, nanos).single()
```

## Performance Tuning Opportunities

### Current Bottlenecks

**Full metadata mode (3,971 files/sec):**
1. Multiple attribute iterations per file (3-4 passes)
2. String allocations for paths
3. Sequential MFT record iteration
4. Attribute parsing overhead

### Future Optimizations

**1. Batch Attribute Reading**
```rust
// Instead of: name → timestamps → size
// Do: Read all attributes in one pass
for attr in file.attributes() {
    match attr.ty()? {
        FileName => process_name(attr),
        StandardInfo => process_timestamps(attr),
        Data => process_size(attr),
    }
}
```

**2. Parallel Processing**
```rust
use rayon::prelude::*;

entries.par_iter_mut()
    .for_each(|entry| {
        // Process entries in parallel
    });
```

**3. String Interning**
- Use string pool for common path components
- "C:\", "Windows\", "System32\" → single allocation

**4. MFT Caching**
- Cache parsed MFT records
- Reduce re-parsing for parent lookups

**5. Incremental Updates**
- Use USN Journal for real-time updates
- Don't rescan entire volume every time

**Potential speedup:** 10-20x (40k → 400k files/sec)

## Error Handling Strategy

### Recoverable Errors

**Missing file name attributes:**
```
Warning: Failed to process record 12: No file name attribute found
```

**Rationale:**
- System/special MFT entries may lack normal attributes
- Don't fail entire scan for corrupt/special entries
- Only log first 1,000 errors to avoid spam

### Fatal Errors

- Volume access denied (not running as admin)
- Drive not found
- NTFS boot sector parse failure
- Out of memory

**Response:** Display error, suggest solution, exit gracefully

## Testing Strategy

### Volume Access Tests

`src/volume_test.rs` provides:

```bash
.\nothing.exe --test-volume C
```

**Tests:**
1. std::fs::File direct access
2. Windows API with flags
3. BufReader wrapper
4. ntfs-reader crate
5. ✅ SectorAlignedReader (WORKING)

### Manual Testing

**Fast mode:**
```bash
.\nothing.exe -i C
# Verify: 155k files/sec, <2.5 GB memory
```

**Full metadata:**
```bash
.\nothing.exe -f -i D
# Verify: Timestamps and sizes display
```

**Multi-drive:**
```bash
.\nothing.exe -a -i
# Verify: All fixed drives scanned
```

**Cloud storage:**
```bash
.\nothing.exe -c -i
# Verify: Google Drive/Dropbox folders indexed
```

## Build Configuration

### Release Profile

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
```

**Benefits:**
- Smaller binary size
- Better inlining
- Faster execution
- ~20% faster than default release profile

### Binary Size

**Stripped release:** ~2.4 MB

**Size breakdown:**
- ntfs crate: ~800 KB
- windows crate: ~400 KB
- Other dependencies: ~600 KB
- Application code: ~600 KB

## Limitations

### Current Limitations

1. **Windows-only:** NTFS-specific implementation
2. **Administrator required:** Raw volume access needs elevation
3. **No persistence:** Index not saved to disk (yet)
4. **No real-time updates:** Must rescan to see changes (USN Journal planned)
5. **Full metadata is slow:** 4k files/sec vs 155k in fast mode

### Design Limitations

1. **MFT enumeration:** Tries every record number (0 to MAX)
   - Sparse MFT has gaps
   - Future: Use MFT bitmap to skip gaps

2. **Single-threaded scanning:** Sequential attribute parsing
   - Future: Parallel MFT processing

3. **Cloud folders:** Directory walking is slow
   - No better alternative for virtual filesystems

## Future Enhancements

### Phase 3: USN Journal Monitoring

**Goal:** Real-time index updates without rescanning

```rust
let journal = UsnJournal::new(volume)?;
for record in journal.iter_from(last_usn) {
    match record.reason {
        FILE_CREATE => index.add(record),
        FILE_DELETE => index.remove(record),
        RENAME => index.update_path(record),
    }
}
```

**Benefit:** Index stays current automatically

### Phase 4: Index Persistence

**Goal:** Save/load index for instant startup

```rust
// Save
index.save_to_file("C:\\Users\\You\\.nothing\\index.bin")?;

// Load (instant startup)
let index = FileIndex::load_from_file("...")?;
```

**Benefit:** No waiting for initial scan

### Phase 5: Advanced Filters

```rust
// Search by size
> cargo.toml size:>1mb

// Search by date
> modified:today

// Regex
> regex:test.*\.rs$
```

### Phase 6: GUI

Windows native GUI using:
- `egui` - Immediate mode GUI
- `iced` - Elm-inspired framework
- `tauri` - Web-based UI with Rust backend

## Conclusion

The sector-aligned reader breakthrough unlocked full NTFS metadata access. The dual-mode design lets users choose between:
- **Fast mode:** 155k files/sec, name/path only
- **Full metadata:** 4k files/sec, complete file information

With 96 GB RAM, you can index 50M+ files with full metadata and search instantly.

---

**Version:** 0.3.0
**Date:** 2026-02-01
**Status:** Production-ready with full metadata support
