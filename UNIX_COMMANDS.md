# Unix-Style Commands on Windows

Ferrum automatically provides Unix-style command wrappers with proper option parsing for Windows.

## Available Commands

### File and Directory Listing

#### `ls [options] [path]`
List directory contents with Unix options support.

**Options:**
- `-a`, `-A` - Show hidden files
- `-l` - Long listing format (default on Windows)
- `-la`, `-al` - Long listing with hidden files
- `-h` - Human-readable sizes (accepted but ignored)
- `--color[=WHEN]` - Colorize output (accepted but ignored, dir is colorized by default)

**Examples:**
```bash
ls
ls -la
ls -a C:\Windows
ls /a          # Windows options also work
```

#### `ll [path]`
Alias for `ls -la`

**Example:**
```bash
ll
ll C:\Projects
```

---

### File Content

#### `cat [options] file...`
Display file contents (supports multiple files).

**Options:**
- `-n` - Number lines (accepted but ignored)

**Examples:**
```bash
cat file.txt
cat file1.txt file2.txt file3.txt
```

#### `head [-n lines] file`
Output first N lines of file (default: 10).

**Options:**
- `-n NUM` - Output first NUM lines

**Examples:**
```bash
head file.txt
head -n 20 file.txt
```

#### `tail [-n lines] file`
Output last N lines of file (default: 10).

**Options:**
- `-n NUM` - Output last NUM lines
- `-f` - Follow (not supported, will show error)

**Examples:**
```bash
tail file.txt
tail -n 50 log.txt
```

#### `wc [options] file`
Count lines, words, or bytes in file.

**Options:**
- `-l` - Count lines (default)
- `-w` - Count words
- `-c` - Count bytes
- `-m` - Count characters

**Examples:**
```bash
wc file.txt          # Show line count
wc -l file.txt       # Lines only
wc -w file.txt       # Words only
wc -c file.txt       # Bytes only
```

---

### File Operations

#### `rm [options] file...`
Remove files or directories.

**Options:**
- `-r`, `-R` - Remove directories recursively
- `-f` - Force (ignore nonexistent files)
- `-rf`, `-fr` - Recursive and force

**Examples:**
```bash
rm file.txt
rm -rf directory/
rm -f *.tmp
```

#### `cp source dest`
Copy files.

**Examples:**
```bash
cp file.txt backup.txt
cp file.txt C:\Backup\
```

#### `mv source dest`
Move/rename files.

**Examples:**
```bash
mv old.txt new.txt
mv file.txt C:\Documents\
```

#### `touch file...`
Create empty files.

**Examples:**
```bash
touch file.txt
touch file1.txt file2.txt
```

---

### Directory Operations

#### `pwd`
Print working directory.

**Example:**
```bash
pwd
```

#### `mkdir dir...`
Create directories.

**Example:**
```bash
mkdir newdir
mkdir dir1 dir2 dir3
```

#### `rmdir dir`
Remove directories recursively.

**Example:**
```bash
rmdir olddir
```

---

### Search and Text

#### `grep [options] pattern [files...]`
Search for pattern in files.

**Options:**
- `-i` - Case-insensitive search
- `-n` - Show line numbers (accepted but ignored)
- `--color[=WHEN]` - Colorize output (accepted but ignored)

**Examples:**
```bash
grep "error" log.txt
grep -i "warning" *.log
echo "hello" | grep "hello"
```

#### `find [path] [options]`
Find files by name or type.

**Options:**
- `-name pattern` - Search by name pattern
- `-type f` - Files only
- `-type d` - Directories only

**Examples:**
```bash
find . -name "*.txt"
find C:\Projects -name "*.rs"
find . -type d
```

---

### System Information

#### `ps [options]`
List running processes.

**Options:**
- `-e`, `-f`, `aux` - All accepted, shows full process list

**Example:**
```bash
ps
ps aux
```

#### `kill PID`
Kill process by PID.

**Example:**
```bash
kill 1234
```

#### `which command`
Find program location (mapped to `where`).

**Example:**
```bash
which python
which git
```

#### `env`
Show environment variables (mapped to `set`).

**Example:**
```bash
env
```

#### `du [options] [path]`
Disk usage statistics.

**Options:**
- `-h` - Human-readable sizes
- `-s` - Summary only
- `-sh` - Summary with human-readable sizes

**Examples:**
```bash
du
du -sh C:\Projects
```

---

### Misc

#### `clear`
Clear screen (mapped to `cls`).

**Example:**
```bash
clear
```

---

## How It Works

When starting cmd.exe, Ferrum:
1. Creates wrapper scripts in `%TEMP%\ferrum_scripts\` for each command
2. Each wrapper parses Unix-style options and converts them to Windows equivalents
3. Adds the scripts directory to PATH
4. Sets up simple doskey aliases for commands that don't need wrappers

Example wrapper logic for `ls`:
- `ls -la` → parses `-la` → calls `dir /a`
- `ls -a C:\` → parses `-a C:\` → calls `dir /a C:\`

## Usage Examples

```bash
# List files with various options
ls
ls -la
ls -a /w

# Work with file content
cat README.md
head -n 5 log.txt
tail -n 20 error.log
wc -l *.txt

# File operations
rm -rf old_project/
cp important.txt backup.txt
touch newfile.txt

# Search
grep -i "error" *.log
find . -name "*.rs"
find C:\Projects -type d

# System info
ps
du -sh C:\Projects
which python
```

## Supported Options Summary

| Command | Fully Supported Options | Ignored Options | Unsupported Options |
|---------|------------------------|-----------------|---------------------|
| `ls` | `-a`, `-l`, `-la` | `-h`, `--color` | `-R` (use dir /s) |
| `rm` | `-r`, `-f`, `-rf` | - | - |
| `cat` | - | `-n` | - |
| `grep` | `-i` | `-n`, `--color` | `-E`, `-v` |
| `head` | `-n NUM` | - | `-c` |
| `tail` | `-n NUM` | - | `-f` |
| `wc` | `-l`, `-w`, `-c` | - | - |
| `find` | `-name`, `-type` | - | `-exec`, `-mtime` |
| `du` | `-h`, `-s` | - | `-a` |

## Limitations

1. **Not all options**: Only the most common Unix options are supported
2. **Windows behavior**: Commands ultimately use Windows tools, so output format differs from Linux
3. **No complex syntax**: Advanced features like command substitution in find `-exec` not supported
4. **Performance**: Some commands use PowerShell internally (e.g., `tail`, `du -sh`) which may be slower

## For True Unix Experience

If you need full Unix compatibility:
- Use **Git Bash** as your shell in Ferrum
- Or install **WSL** (Windows Subsystem for Linux) and use `wsl bash`
- Or use **PowerShell** with its native Unix-like cmdlets

## Customization

To modify or add wrappers, edit the `create_*_wrapper` functions in `src/pty/mod.rs` and rebuild:

```bash
cargo build --release
```

Each wrapper is a simple batch script that parses options and calls the appropriate Windows command.
