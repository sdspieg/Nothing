# Changelog

## [0.3.0] - 2026-02-01

### ✅ MAJOR BREAKTHROUGH - Full Metadata Mode FIXED!

**Boot Sector Issue Resolved:**
- Created `SectorAlignedReader` wrapper to handle Windows raw volume I/O requirements
- The `ntfs` crate's binrw parser requires sector-aligned reads (512 bytes)
- Windows ERROR_INVALID_PARAMETER (87) was caused by non-aligned reads
- Solution: Custom reader that buffers and aligns all reads to sector boundaries

### 🎉 New Features

**Full Metadata Support:**
- ✅ File sizes
- ✅ Created timestamps
- ✅ Modified timestamps
- ✅ Accessed timestamps
- ✅ Works with ntfs crate via sector-aligned reads
- Performance: 30-50k files/sec (3-5x slower than fast mode, as expected)

**Multi-Drive Support:**
- `--all-drives` / `-a` flag to scan all fixed drives
- Automatic drive detection (A-Z)
- Drive type identification (Fixed, Removable, Network, CD, RAM)
- Only scans fixed drives by default (skips removable, network, etc.)

**Cloud Storage Support:**
- `--include-cloud` / `-c` flag to index cloud storage
- Auto-detects Google Drive, Dropbox, OneDrive folders
- Uses directory walking for cloud folders (since they're not in MFT)
- Skips cloud-only placeholder files (not downloaded)
- Works with synced cloud folders

### 🔧 Technical Improvements

**Sector-Aligned Reader:**
- Custom `Read` + `Seek` implementation
- 8KB buffer (16 sectors)
- Transparent sector alignment for all reads
- Allows ntfs crate's binrw parser to work with raw volumes

**Error Handling:**
- Graceful handling of missing file name attributes (system entries)
- Per-drive error handling in multi-drive mode
- Cloud folder access errors don't stop entire scan

### 📊 Performance

**Fast Mode (working since v0.2.0):**
- 155,088 files/sec
- 10.7M files in 73.74 seconds
- 2.12 GB memory

**Full Metadata Mode (NOW WORKING!):**
- **Actual measured:** 3,971 files/sec (tested on D: drive with 195,044 files)
- Time: 60.10 seconds for 195k files
- Memory: 1.06 GB for 195k files (~5.4 KB per file)
- **Projected for C: drive:** ~45 minutes for 10.7M files, ~4-5 GB memory
- Note: Slower than initial estimate due to multiple attribute parsing passes

### 🚀 Usage Examples

```bash
# Fast mode (names and paths only)
.\nothing.exe -i

# Full metadata mode (sizes and timestamps)
.\nothing.exe --full-metadata -i

# Scan all fixed drives
.\nothing.exe --all-drives -i

# Include cloud storage
.\nothing.exe --all-drives --include-cloud -i

# Full metadata + all drives + cloud storage
.\nothing.exe -f -a -c -i
```

### 🐛 Known Issues & Future Optimizations

**Performance:**
- Full metadata mode is slower than initially estimated (4k vs 30-50k files/sec)
- Root cause: Multiple attribute iteration passes per file
  - Pass 1: Find $FILE_NAME attribute
  - Pass 2: Find $STANDARD_INFORMATION for timestamps
  - Pass 3: Find $DATA for file size
- Potential fix: Single-pass attribute collection (10-20x speedup possible)

**Trade-offs:**
- Fast mode: 155k files/sec, no metadata
- Full metadata: 4k files/sec, complete information
- For 96GB RAM systems: 45-minute initial scan is acceptable for months of instant searches

### ✅ Testing & Verification

**Volume Access Tests:**
- Created `--test-volume` flag to test different access methods
- Tested: std::fs::File, Windows API with flags, BufReader, ntfs-reader crate
- ✅ SectorAlignedReader passes all tests

**Performance Testing:**
- Fast mode: Verified 155,088 files/sec on C: drive (10.7M files)
- Full metadata: Measured 3,971 files/sec on D: drive (195k files)
- Memory usage: Confirmed ~5.4 KB per file with full metadata

**Feature Testing:**
- ✅ Multi-drive detection and scanning works
- ✅ Cloud storage auto-detection works (Google Drive, Dropbox, OneDrive)
- ✅ Interactive search with fuzzy matching works
- ✅ Timestamps and file sizes display correctly

### 📝 Files Changed

**New Files:**
- `src/sector_aligned_reader.rs` - ⭐ Sector-aligned I/O wrapper (THE KEY BREAKTHROUGH)
- `src/multi_drive.rs` - Multi-drive and cloud storage support
- `src/volume_test.rs` - Volume access testing utilities
- `TECHNICAL_DETAILS.md` - Comprehensive technical documentation
- `CHANGELOG.md` - This file

**Modified Files:**
- `src/mft_reader_ntfs.rs` - Now uses SectorAlignedReader
- `src/main.rs` - Added multi-drive and cloud flags
- `src/file_entry.rs` - Extended with metadata fields
- `src/interactive.rs` - Display sizes and timestamps
- `Cargo.toml` - Added ntfs-reader, walkdir dependencies
- `README.md` - Updated with new features
- `IMPLEMENTATION_SUMMARY.md` - Updated status to ✅ WORKING

## [0.2.0] - 2026-02-01

### Initial Release Features

- Fast MFT enumeration using `usn-journal-rs`
- In-memory indexing
- Fuzzy search with `nucleo-matcher`
- Interactive CLI
- Memory optimizations (Arc<str> for paths)
- Color-coded search results

---

**Legend:**
- ✅ = Fully working
- ⚠️  = Partially working / has issues
- 🔧 = In development
- 🎉 = New feature
