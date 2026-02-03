# ⚠️ CRITICAL SAFETY WARNING ⚠️

## Protecting Your Index Files

Your index files in `~/.nothing/` contain **MILLIONS of file entries** representing hours of MFT scanning.

### 🛡️ Safety Protections in Place:

1. **`create_test_index` binary** - DELETED from release builds
   - This tool creates FAKE test data
   - It would **OVERWRITE** your real index
   - Only use for testing on empty systems

2. **Safety check added** - Will refuse to run if real index exists
   - Checks if `index_*.bin` > 100 KB
   - Aborts with error message
   - Prevents accidental data loss

### ❌ NEVER DO THIS:

```bash
# DON'T run the test index creator if you have real data!
cargo run --bin create_test_index  # ⚠️ DANGER!
```

### ✅ Safe Operations:

```bash
# Safe: Run normal MFT scan (creates/updates index)
nothing.exe C -f

# Safe: Run all drives scan
nothing.exe -a -f

# Safe: Launch GUI (reads index, doesn't overwrite)
nothing.exe --gui

# Safe: View index contents
cargo run --bin show_index
```

### 📍 Index File Locations:

- `~/.nothing/index_C.bin` - C: drive index (e.g., 2.6 GB, 12M files)
- `~/.nothing/index_D.bin` - D: drive index (if exists)
- etc.

### 🔄 When to Rescan:

Your index is automatically kept up-to-date by:
- **USN Journal monitoring** (real-time updates while GUI is running)
- **Manual rescan** (run `nothing.exe -a -f` when needed)

### 💾 Backup Your Index (Optional):

```bash
# Backup your index files
cp ~/.nothing/*.bin ~/index_backup/

# Restore if needed
cp ~/index_backup/*.bin ~/.nothing/
```

## Current Index Status:

Your index contains **millions of file entries** from complete MFT scans.

**NEVER overwrite this with test data!**
