# Nothing - Fast Windows File Search Tool

A lightweight, fast Windows file search tool written in Rust that reads the NTFS Master File Table (MFT) directly for rapid indexing.

## Features

- **Fast MFT Scanning**: Enumerates all files on an NTFS volume in seconds (155k files/sec)
- **Full Metadata Support**: ✨ File sizes and timestamps (created, modified, accessed)
- **Real-Time Monitoring**: 🔥 **NEW!** Index auto-updates as files change (enabled in interactive mode)
- **Index Persistence**: 💾 **NEW!** Save/load index to disk for instant startup
- **Multi-Drive Scanning**: 🎉 Scan all fixed drives automatically with `--all-drives`
- **Cloud Storage Integration**: 🎉 Index Google Drive, Dropbox, OneDrive with `--include-cloud`
- **In-Memory Indexing**: Stores file paths and metadata for instant access
- **Fuzzy Search**: Typo-tolerant search powered by nucleo-matcher
- **Interactive CLI**: Real-time search results as you type
- **Memory Optimized**: Efficient string storage (2-4 GB for 10M+ files)
- **Color-Coded Results**: File/directory distinction with relevance scores

## Requirements

- Windows 10/11
- Administrator privileges (required for raw volume access)
- NTFS filesystem

## Installation

### Build from source

```bash
cargo build --release
```

The compiled binary will be at `target\release\nothing.exe`

## Usage

**Important**: You must run this tool as Administrator.

### Scan and enter interactive search mode

```bash
nothing.exe --interactive
```

Or using the short flag:

```bash
nothing.exe -i
```

### Just scan without searching

```bash
nothing.exe
```

### Scan a specific drive

```bash
nothing.exe D -i
```

### Full metadata mode (includes file sizes and timestamps)

```bash
nothing.exe --full-metadata -i
```

Or using the short flag:

```bash
nothing.exe -f -i
```

### Scan all fixed drives

```bash
nothing.exe --all-drives -i
```

Or:

```bash
nothing.exe -a -i
```

### Include cloud storage folders (Google Drive, Dropbox, OneDrive)

```bash
nothing.exe --include-cloud -i
```

Or:

```bash
nothing.exe -c -i
```

### Combine all features

```bash
# Full metadata + all drives + cloud storage + interactive search
nothing.exe -f -a -c -i
```

### Run as Administrator

Right-click PowerShell and select "Run as Administrator", then:

```bash
cd C:\Apps\Nothing
.\target\release\nothing.exe -i
```

## Interactive Search

Once in interactive mode:
- **Type** to search - fuzzy matching finds files even with typos
- **Results update instantly** as you type
- **Top 50 results** shown, sorted by relevance
- **Total match count** displayed
- **Ctrl+C or Ctrl+D** to exit

### Search Features

- **Fuzzy matching**: Typo-tolerant search (e.g., "dcuments" finds "Documents")
- **Filename priority**: Matches in filenames rank higher than path matches
- **Case-insensitive**: Search works regardless of capitalization
- **Instant results**: Searches entire index in milliseconds

## Example Output

### First Run (Full Scan)

```
Nothing - Fast File Search Tool
No cached index found, performing full scan...
Scanning drive C:...

Progress: 100,000 files...
Progress: 200,000 files...
...
Progress: 10,700,000 files...

Scan complete!
Total files: 10,726,987
Directories: 590,467
Time taken: 73.74 seconds (155,088 files/sec)
Index memory: 2.12 GB

Saving index to disk...
✅ Index saved

Entering interactive search mode with real-time monitoring...
📡 Monitoring drive C:\
```

### Subsequent Runs (Cached)

```
✅ Loaded cached index from disk
   10,726,987 files, 590,467 directories

Entering interactive search mode with real-time monitoring...
📡 Monitoring drive C:\
```

### Interactive Search

```
Nothing - Interactive Search (Real-Time Monitoring)
📡 Index auto-updates as files change
Press Ctrl+C or Ctrl+D to exit
Type to search (fuzzy matching enabled)...

> readme

Found 1,234 matches (showing top 50)

 1. [FILE] README.md
    C:\Apps\Nothing\README.md (score: 2840)
 2. [FILE] readme.txt
    C:\Users\Documents\readme.txt (score: 2800)
 3. [DIR] ReadMe
    C:\Projects\ReadMe (score: 2600)
...
```

## What's Working

- ✅ File and directory names
- ✅ Full paths
- ✅ Fast enumeration (155k+ files/sec)
- ✅ Interactive fuzzy search
- ✅ Real-time results
- ✅ **Real-time monitoring** (auto-updates on file changes)
- ✅ **Index persistence** (save/load from disk)
- ✅ File sizes and timestamps (with `--full-metadata`)
- ✅ Multi-drive and cloud storage support

## Roadmap

### Phase 1: Fast MFT Scanning ✅ COMPLETE
- MFT enumeration with usn-journal-rs
- In-memory indexing
- Full path reconstruction

### Phase 2: Full Metadata & Search ✅ COMPLETE
- File sizes and timestamps
- Interactive fuzzy search
- Multi-drive and cloud storage support

### Phase 3: Real-Time Monitoring ✅ COMPLETE
- ✅ Real-time file change detection using `notify` crate
- ✅ Automatic index updates
- ✅ Index persistence (save/load to disk)
- ✅ Cloud storage monitoring

### Phase 4: Advanced Features (Future)
- Search filters (size, date range, extension)
- Export results to CSV/JSON
- Search history
- Performance metrics dashboard

### Phase 5: GUI (Future)
- Native Windows GUI
- Better visualization
- Search result management

## Architecture

```
src/
├── main.rs          # CLI entry point
├── mft_reader.rs    # MFT enumeration logic
├── file_entry.rs    # File metadata structures
├── index.rs         # In-memory file index
└── error.rs         # Error handling
```

## Technical Details

- Uses Windows `FSCTL_ENUM_USN_DATA` to enumerate MFT entries
- Builds full paths by tracking parent directory references
- Stores entries in a Vec with HashMap for path reconstruction
- Progress updates every 100,000 files

## Performance

Expected performance on modern hardware:
- **SSD**: 500k-1M+ files/second
- **HDD**: 100k-500k files/second
- **Memory usage**: ~50-150 bytes per file (varies with path length)

For 1 million files:
- Scan time: 1-10 seconds
- Memory usage: 50-150 MB

## License

MIT

## Contributing

Contributions welcome! Areas for improvement:
- Add search functionality
- Implement USN journal monitoring
- Add full MFT metadata support (sizes, timestamps)
- GUI development
- Performance optimizations

## Acknowledgments

Built with:
- [usn-journal-rs](https://github.com/wangfu91/usn-journal-rs) - USN journal and MFT enumeration
- [clap](https://github.com/clap-rs/clap) - Command-line parsing
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling
