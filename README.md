# port

A simple TUI application for managing open network ports on Linux.

<img width="941" height="508" alt="image" src="https://github.com/user-attachments/assets/fae569fe-3343-4b09-87e7-a34792033778" />

## Problem

You run an application, the terminal crashes for some reason, and the server keeps running in the background. To find and kill the process responsible for that port, you need to run non-trivial commands. This tool solves that.

## Solution

A Rust CLI with a terminal UI (TUI) that lists open ports, filters out system services, and lets you kill processes with a few keystrokes.

## Features

- **List open ports** — Shows all TCP ports with process name and executable path
- **Kill by port** — CLI mode to directly kill a process on a given port
- **Search** — Live filter by process name or port number
- **Kill** — Enter to open confirmation modal, then confirm to force kill (SIGKILL)
- **Keyboard-driven** — No mouse needed

## Installation

### Pre-built binaries (recommended)

```bash
curl -LsSf https://raw.githubusercontent.com/enrell/port/main/scripts/install.sh | sh
```

This downloads and installs the latest release for your architecture (x86_64 or aarch64).

### From source

```bash
cargo build --release
install -Dm755 target/release/port ~/.local/bin/port
```

## Usage

```bash
port              # Start TUI (lists all open ports)
port 3000         # Kill the process on port 3000
port -t           # Explicit TUI mode (same as bare 'port')
```

### Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `/` or `i` | Enter search mode |
| `Enter` | Open kill confirmation modal |
| `y` | Confirm kill (in modal) |
| `n` or `Esc` | Cancel (in modal) |
| `r` | Refresh port list |
| `q` or `Ctrl+C` | Quit |

## Architecture

```
src/
├── main.rs      # Entry point
├── app.rs       # State management
├── ui.rs        # Ratatui rendering
├── events.rs    # Input handling
├── ports.rs     # Port discovery (/proc/net/tcp)
├── process.rs   # Process operations (kill)
├── filter.rs    # System port filtering
└── lib.rs       # Module exports
```

## Development

Run tests:
```bash
cargo test
```

Run in debug mode:
```bash
cargo run
```

## Requirements

- Linux kernel (reads from /proc filesystem)
- Rust toolchain

## License

[MIT](LICENSE)
