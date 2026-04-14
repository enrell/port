# AGENTS.md

Technical documentation for AI agents working on the `port` codebase.

---

# Overview

A Rust TUI (Terminal User Interface) tool for managing open network ports on Linux. The core purpose: list user-owned processes listening on TCP ports, filter out system services, and allow quick process termination.

The problem it solves: a crashed terminal leaves a server running in the background, and finding/killing the process requires running non-trivial shell commands.

---

# Architecture

## Module Structure

```
src/
├── main.rs      # Entry point: crossterm init, terminal setup, main loop
├── app.rs       # State machine: ports list, filter state, selected index, mode
├── events.rs    # Input handling: key event dispatch based on current mode
├── ui.rs        # Ratatui rendering: table, header, footer, modal dialogs
├── ports.rs     # Port discovery: parse /proc/net/tcp, map inodes to PIDs
├── process.rs   # Process operations: SIGKILL via libc::kill
├── filter.rs    # System port filtering: hardcoded blacklist
└── lib.rs       # Module exports (for tests and lib usage)
```

## Data Flow

```
 get_open_ports()              App::refresh_ports()
       ┃                              ┃
       ▼                              ▼
  ┌──────────┐  filter via       ┌──────────┐
  │ PortInfo │ ─should_include_─▶│ App.ports│
  │  (Vec)   │  port()           │  (Vec)   │
  └──────────┘                   └──────────┘
       ┃                               ┃
       ▼                               ▼
 /proc/net/tcp                    apply_filter()
 /proc/[pid]/fd/                       ┃
       ┃                          ┌────┴────┐
       ▼                          ▼         ▼
 parse_port_ADDR                  PortInfo───▶ ui::render_table()
 build_inode_pid_map                     (filtered Vec<usize>)
```

## How Ports Are Discovered

**File: `src/ports.rs`**

1. **Build inode→PID map**: Iterate `/proc/[pid]/fd/` reading symlinks.
   - Socket symlinks look like `socket:[12345]` — extract the inode number
   - Store `HashMap<inode, pid>` for quick lookup

2. **Parse `/proc/net/tcp` (and tcp6)**:
   - Skip header line
   - Extract: local address hex, remote address, state (hex "0A" = LISTEN), inode
   - Parse port from hex address: last 4 hex chars → `u16`
   - State `0A` (LISTEN) required; skip other states

3. **Match inode to process**: Look up inode in the map, get PID.

4. **Get process info**: Read `/proc/[pid]/comm` for name, `/proc/[pid]/exe` symlink for path.

5. **Return**: `Vec<PortInfo>` sorted by port number ascending.

## How Socket Inodes Map to PIDs

In Linux, each socket has an inode in the kernel. `/proc/net/tcp` shows the inode owning the socket. `/proc/[pid]/fd/[N]` symlinks reveal which inodes each process holds.

The algorithm in `build_inode_pid_map()`:
```rust
for each /proc/[pid]/fd/*:
    if symlink_target starts with "socket:[":
        extract inode number
        map[inode] = pid
```

This is O(processes × fds) but completes in <100ms on typical systems.

## Filtering Logic

**File: `src/filter.rs`**

Two-tier blacklist approach (hardcoded constants):

1. **Port-based**: `BLOCKED_PORTS = [22, 80, 443, 53, 3306, 5432, 6379, 8080, 3000, 5000, 8000, 9000]`
   - These are common system/development ports we don't want to accidentally kill

2. **Process-based**: `BLOCKED_PROCESSES = ["systemd", "dockerd", "containerd", "sshd", "tor", "docker", "kubelet", "kube-proxy"]`
   - Substring match (case-insensitive) so `/usr/bin/dockerd` matches `dockerd`

`should_include_port(port, name)` returns `true` only if neither blocked.

## TUI State Machine

**File: `src/app.rs` — Mode enum:**

```rust
pub enum Mode {
    Normal,                        // Default: navigate, kill, refresh
    Search,                        // Typing filters the list
    ConfirmKill { pid: u32, name: String },  // Y/N confirmation modal
}
```

**Transitions:**
- `/` or `i` in Normal → Search
- `Enter` in Normal → ConfirmKill (if port selected)
- `Esc` or `Enter` in Search → Normal
- `y` in ConfirmKill → execute_kill() → Normal
- `n` or `Esc` in ConfirmKill → Normal

The `App` struct holds the canonical state:
- `ports: Vec<PortInfo>` — all discovered ports (post-filtering)
- `filtered: Vec<usize>` — indices into `ports` matching search query
- `selected: usize` — cursor position into `filtered`
- `search_query: String` — active filter text

---

# Key Design Decisions

## Why /proc parsing instead of lsof

- **Zero external dependencies**: No shelling out, no parsing text output
- **Performance**: Direct file reads are faster than fork+exec of lsof
- **Control**: Can access exactly the fields we need (inode, comm, exe)
- **Transparency**: User can see exactly what we're doing

Trade-off: Linux-only, requires understanding /proc format.

## Why SIGKILL vs SIGTERM

**File: `src/process.rs`** — uses `libc::SIGKILL`

- The tool's purpose is force-killing orphaned/zombie processes
- SIGTERM can be caught/blocked; stuck processes often ignore it
- SIGKILL is immediate and reliable for the target use case
- Confirmation modal provides safety from accidental triggers

## Why This Blacklist Approach

Simple and opinionated:
- Hardcoded lists → predictable, no config files to manage
- Covers 90% of "don't accidentally kill this" cases
- Can be extended to config file later if needed
- Explicit over implicit: users see what's excluded

## Tradeoff: sysinfo vs manual /proc for process info

We use **manual /proc parsing** in `ports.rs` (not the `sysinfo` crate) for:
- Port-to-process mapping (requires inode tracking)
- Fast refresh without building full process tables

We use **sysinfo** only if needed elsewhere (currently not used for port discovery, but available in deps for future extensions).

---

# Extending

## Adding New Columns to the Table

**Files to modify:**
1. `src/ports.rs` — add field to `PortInfo` struct
2. `src/ports.rs` — populate field in `get_process_info()` or parsing
3. `src/ui.rs` — add header in `render_table()` Line ~58
4. `src/ui.rs` — add cell in the `Row::new(vec![...])` around Line 80-85
5. `src/ui.rs` — adjust `Constraint::` widths table around Line 90-95

## Adding UDP Support

**Files to modify:**
1. `src/ports.rs` — `get_open_ports()` already parses tcp6; add `/proc/net/udp` and `/proc/net/udp6`
2. Reuse `parse_tcp_file()` — rename to `parse_socket_file()` or duplicate for UDP
3. UDP state codes differ (no LISTEN in same sense); accept all bound sockets
4. `PortInfo` may need a `protocol: Protocol` enum field
5. Update `ui.rs` table to show Protocol column

## Making Filters Configurable

**Approach:**
1. Add CLI args (using `clap` crate) or config file (toml/yaml)
2. Create `FilterConfig` struct with port list, process list
3. Pass config to `filter::should_include_port()` — replace hardcoded constants
4. Extend `App` to hold `FilterConfig`

**Entry points:**
- `src/main.rs` — parse args, load config
- `src/app.rs` — store config, pass to refresh_ports()
- `src/filter.rs` — accept config parameter

---

# Testing

## Test Structure

Tests are colocated in each module using `#[cfg(test)]`:

```
src/
├── app.rs      # Mode transition tests, navigation tests, search filter tests
├── events.rs   # Key handler tests (quit, nav, search typing, confirm)
├── ports.rs    # parse_port() unit tests
├── process.rs  # kill_process() error handling test (nonexistent PID)
└── filter.rs   # Blocked port/process logic tests
```

## Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test events::

# With output visible
cargo test -- --nocapture

# Single test
cargo test test_navigation -- --nocapture
```

## Where to Add New Tests

- **Logic changes**: Add to same file in `#[cfg(test)]` mod
- **Integration tests**: Create `tests/` directory at project root (currently none exist)
- **UI tests**: Ratatui has `Backend::test()` for snapshot testing; not currently used

## Test Helpers

Common pattern in `app.rs` tests:
```rust
fn create_test_port(port: u16, pid: u32, name: &str) -> PortInfo {
    PortInfo { port, pid, process_name: name.to_string(), process_path: "/bin/test".to_string() }
}
```

---

# Constraints

## Platform Requirements

- **Linux only**: Requires `/proc` filesystem with specific format
- **Kernel**: Any modern Linux (tested on 5.x+)
- **Not portable**: Will not compile/run on macOS, BSD, Windows without major rewrite

## Permission Requirements

- **Port discovery**: No special permissions (reads public /proc)
- **Kill process**: Requires permission to signal target process:
  - Own processes: always works
  - Other user processes: requires root or appropriate capabilities
- **Error handling**: EPERM (permission denied) and ESRCH (process gone) are gracefully handled

## Resource Constraints

- **Memory**: Holds full port list in memory (typically <1MB)
- **CPU**: /proc scanning is O(processes × fds); acceptable for interactive use
- **Refresh rate**: No auto-refresh; manual `r` key only

## Dependencies

```toml
ratatui = "0.28"    # TUI framework
crossterm = "0.28"  # Cross-platform terminal control
sysinfo = "0.30"    # Process info (currently unused, available for extensions)
libc = "0.2"        # kill(), signal constants
```

## Code Conventions

- **Error handling**: `io::Result` propagation, `?` operator preferred
- **String handling**: `to_string_lossy()` for /proc paths (may be invalid UTF-8)
- **Case sensitivity**: Search uses `to_lowercase()`; process matching uses `to_lowercase()`
- **Cloning**: Explicit `.clone()` calls; no implicit copies
