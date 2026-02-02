# Nothing - Fast Windows File Search Tool

A lightweight, fast Windows file search tool written in Rust that reads the NTFS Master File Table (MFT) directly for rapid indexing.

## Features

### Core Search & Indexing
- **Fast MFT Scanning**: Enumerates all files on an NTFS volume in seconds (155k files/sec)
- **Full Metadata Support**: ✨ File sizes and timestamps (created, modified, accessed)
- **Real-Time Monitoring**: 🔥 Index auto-updates as files change (enabled in interactive mode)
- **Index Persistence**: 💾 Save/load index to disk for instant startup (<10 seconds)
- **Multi-Drive Scanning**: 🎉 Scan all fixed drives automatically with `--all-drives`
- **Cloud Storage Integration**: 🎉 Index Google Drive, Dropbox, OneDrive with `--include-cloud`

### Phase 4: Advanced Search Features ✨ **NEW!**
- **Advanced Filters**: Size, extension, date range, type filtering
  - `size:>100mb` - Files larger than 100MB
  - `ext:rs,md` - Filter by extensions
  - `modified:7d` - Modified in last 7 days
  - `type:file` - Files or directories only
- **Export Results**: Export to CSV or JSON (Ctrl+E)
- **Search History**: Navigate with ↑/↓ arrows, persists across sessions
- **Performance Metrics**: Real-time stats (F2 to toggle)
- **Interactive Help**: F1 for comprehensive help panel

### User Experience
- **Fuzzy Search**: Typo-tolerant search powered by nucleo-matcher
- **Interactive CLI**: Real-time search results as you type
- **Memory Optimized**: Efficient string storage (2-4 GB for 10M+ files)
- **Color-Coded Results**: File/directory distinction with relevance scores
- **Keyboard Shortcuts**: Full keyboard control for power users

## Recent Updates (v0.6.1)

### Critical Bug Fixes
- **File Path Resolution**: Fixed incorrect file paths where files appeared at root level (e.g., `C:\filename`) instead of their actual location. Implemented two-pass MFT scanning for accurate path resolution.
- **UI Responsiveness**: Fixed GUI freezing when typing quickly. Search requests now properly cancel outdated queries, providing smooth typing experience.

### Important: Index Rebuild Required
If you experience "Windows cannot find" errors when opening files, you need to rebuild your indexes:

```bash
# Delete cached indexes
del %USERPROFILE%\.nothing\*.bin

# Rescan drives (run as Administrator)
nothing.exe --gui
# or
nothing.exe --all-drives --interactive
```

The new two-pass scanning algorithm ensures all file paths are correctly resolved.

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

### GUI Mode (NEW!) 🎨

```bash
# Launch beautiful GUI application
nothing.exe --gui
# or
nothing.exe -g
```

**Features:**
- Dark/Light theme toggle (🌙/☀️)
- Real-time search
- Sortable results table
- Click column headers to sort
- Professional modern interface

### Run as Administrator

Right-click PowerShell and select "Run as Administrator", then:

```bash
cd C:\Apps\Nothing
# GUI mode
.\target\release\nothing.exe -g

# Or CLI mode
.\target\release\nothing.exe -i
```

## Interactive Search

Once in interactive mode:
- **Type** to search - fuzzy matching finds files even with typos
- **Results update instantly** as you type
- **Top 50 results** shown, sorted by relevance
- **Total match count** displayed

### Search Features

**Basic Search:**
- **Fuzzy matching**: Typo-tolerant search (e.g., "dcuments" finds "Documents")
- **Filename priority**: Matches in filenames rank higher than path matches
- **Case-insensitive**: Search works regardless of capitalization
- **Instant results**: Searches entire index in milliseconds

**Advanced Filters (Phase 4):**

Size filters:
```
size:>100mb          # Files larger than 100MB
size:<1gb            # Files smaller than 1GB
size:100kb-500kb     # Files between 100KB and 500KB
```

Extension filters:
```
ext:rs,md,txt        # Files with .rs, .md, or .txt extensions
ext:mp4,avi          # Video files
```

Date filters:
```
modified:7d          # Modified in last 7 days
modified:>2024-01-01 # Modified after Jan 1, 2024
modified:<2024-12-31 # Modified before Dec 31, 2024
created:30d          # Created in last 30 days
```

Type filters:
```
type:file            # Files only
type:dir             # Directories only
```

Combine multiple filters:
```
video size:>100mb modified:7d        # Large recent videos
readme ext:md size:>10kb             # README files in markdown
config type:file modified:30d        # Config files modified last month
```

### Keyboard Shortcuts

- **↑/↓** - Navigate search history
- **Ctrl+E** - Export current results to CSV/JSON
- **F1** - Toggle help panel
- **F2** - Toggle performance statistics
- **Ctrl+C or Ctrl+D** - Exit

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

### Phase 4: Advanced Search Features ✅ COMPLETE
- ✅ Advanced search filters (size, date range, extension, type)
- ✅ Export results to CSV/JSON
- ✅ Search history with arrow key navigation
- ✅ Performance metrics dashboard
- ✅ Interactive help system

### Phase 5: Modern GUI ✅ COMPLETE
- ✅ Beautiful desktop application (iced framework)
- ✅ Dark/Light theme toggle
- ✅ Real-time search interface
- ✅ Sortable results table
- ✅ Professional UI/UX

### Phase 5.1: GUI Enhancements (Future)
- Visual filter builders
- Context menus and drag-and-drop
- Export dialog in GUI
- Settings panel
- Charts and visualizations

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
