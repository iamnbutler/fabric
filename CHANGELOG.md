# Changelog

All notable changes to Spool will be documented in this file.

## [0.3.0] - 2026-01-19

### Added
- Repository field in Cargo.toml for crates.io linking

### Changed
- Refactored lint suppressions into proper named structs (`TaskIndexBuilder`, `AddArgs`, `ListArgs`)
- Consolidated `scripts/` into `script/` directory
- Renamed misleading `md5_hash()` to `simple_hash()` in concurrency module

### Fixed
- Removed duplicate `get_current_branch()` function (now imports from writer module)
- Removed unused `_seen_ids` parameter from validation
- Updated deprecated `actions-rs/toolchain` to `dtolnay/rust-toolchain` in CI
- Added missing `SetStream` operation test coverage
- Fixed benchmark Task creation to include `stream` field

## [0.2.0] - 2026-01-18

### Added
- **Streams**: Group tasks into collections with `--stream` flag
  - `spool add "Task" --stream my-stream` - Create task in a stream
  - `spool list --stream my-stream` - Filter tasks by stream
  - `set_stream` operation to move tasks between streams
- **CLI commands** promoted from shell-only:
  - `spool add` - Create tasks directly from CLI
  - `spool assign <id> @user` - Assign task to a user
  - `spool claim <id>` - Assign task to yourself
  - `spool free <id>` - Unassign task
- `CreateTaskParams` struct for cleaner task creation API

### Changed
- Shell now uses named argument structs instead of tuples

## [0.1.0] - 2026-01-13

### Added

Initial release of Spool, a git-native task management system.

#### Core Features

- **Event-sourced architecture**: All task data stored as append-only JSONL event logs
- **Git-native**: Events are committed with your code, enabling branch-aware workflows
- **Full history**: Every change to every task is preserved and auditable

#### Commands

- `spool init` - Initialize `.spool/` directory structure in a repository
- `spool list` - List tasks with filtering by status, assignee, tag, or priority
- `spool show` - Display detailed task information with optional event history
- `spool rebuild` - Regenerate index and state caches from event logs
- `spool archive` - Move completed tasks to monthly archive files
- `spool validate` - Check event files for correctness and consistency

#### Task Operations Supported

- Create tasks with title, description, priority, assignee, and tags
- Update task metadata
- Assign/unassign tasks
- Add comments with optional references
- Link tasks (blocks, blocked_by, parent relationships)
- Complete and reopen tasks
- Archive old completed tasks

#### Output Formats

- Table format (default) for human-readable output
- JSON format for programmatic access
- IDs-only format for shell scripting

### Technical Details

- Written in Rust for performance and reliability
- Uses serde for JSON serialization
- Integrates with git for branch detection and author identification
- Generates unique task IDs using timestamp + random suffix

### Platforms

- macOS (x86_64, aarch64)
- Linux (x86_64)

### Documentation

- Comprehensive CLI user guide
- Event schema reference
- Git integration best practices
