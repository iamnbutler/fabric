# spool-ui

Terminal UI for [spool](https://crates.io/crates/spool) - git-native task management.

```bash
cargo install spool-ui
```

## Usage

Run in a spool-initialized directory:

```bash
spool-ui
```

## Keybindings

### Task View

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate tasks (or scroll detail when focused) |
| `g` / `G` | First / last task |
| `Enter` | Toggle detail panel |
| `Tab` | Switch focus (list/detail) |
| `c` | Complete task |
| `r` | Reopen task |
| `n` | New task |
| `v` | Cycle view (Open/Complete/All) |
| `s` | Cycle sort (Priority/Created/Title) |
| `S` | Cycle stream filter |
| `/` | Search |
| `h` | Open history view |
| `q` | Quit |

### History View

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate events (or scroll detail when open) |
| `g` / `G` | First / last event |
| `l` / `Left` | Scroll columns horizontally |
| `Enter` | Toggle detail panel |
| `Tab` / `Shift-Tab` | Navigate events when detail is open |
| `Esc` | Close detail / return to tasks |
| `h` | Return to task view |
| `q` | Quit |

## Features

- Task list with priority coloring and status markers
- Detail panel with full task info and event history
- History view showing all events in reverse chronological order
- Status filtering (open/complete/all)
- Sorting (priority/created/title)
- Search (title, description, tags)
- Stream navigation
- Inline task creation and completion

## License

MIT
