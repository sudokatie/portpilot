# portpilot

Cross-platform CLI tool for inspecting and managing ports.

## Installation

```bash
cargo install --path .
```

## Usage

### List all listening ports

```bash
portpilot
```

### Check a specific port

```bash
portpilot 3000
```

### Check a port range

```bash
portpilot 3000-3010
```

### Kill process on a port

```bash
portpilot 3000 --kill
```

### Filter by process name

```bash
portpilot --filter node
```

### Show external ports only

```bash
portpilot --external
```

### JSON output

```bash
portpilot --json
```

### Watch mode (live updates)

```bash
portpilot --watch
```

### Interactive TUI

```bash
portpilot --tui
```

## Exit Codes

- `0` - Port is in use (or listing succeeded)
- `1` - Port is not in use, or error occurred

## Flags

| Flag | Description |
|------|-------------|
| `-j, --json` | Output as JSON |
| `-q, --quiet` | Exit code only, no output |
| `-1, --oneline` | Minimal single-line output |
| `-u, --udp` | Include UDP ports |
| `-f, --filter` | Filter by process name |
| `-e, --external` | Only external ports (0.0.0.0) |
| `-l, --local` | Only localhost ports |
| `-k, --kill` | Kill process using the port |
| `--force` | Use SIGKILL immediately |
| `--wait` | Block until port is free |
| `-w, --watch` | Live-updating display |
| `--tui` | Interactive TUI mode |

## License

MIT
