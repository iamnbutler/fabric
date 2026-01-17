# Getting Started with Fabric

Fabric is a git-native task management system. Tasks are stored as events in your repository, giving you full history and branch-aware workflows.

## Installation

```bash
cargo install fabric
```

Or build from source:
```bash
git clone https://github.com/your-username/fabric.git
cd fabric
cargo install --path .
```

## Quick Start

### 1. Initialize Fabric

In your project directory:

```bash
fabric init
```

This creates `.fabric/` with:
- `events/` - Daily event logs (committed to git)
- `archive/` - Monthly rollups of completed tasks
- `.gitignore` - Ignores derived cache files

### 2. Create a Task

```bash
fabric add "Implement user authentication" -p p1 -t feature
```

Options:
- `-p, --priority` - Priority level (p0, p1, p2, p3)
- `-t, --tag` - Add tags (can use multiple times)
- `-d, --description` - Task description
- `-a, --assignee` - Assign to someone (@username)

### 3. List Tasks

```bash
# List open tasks (default)
fabric list

# List all tasks
fabric list --status all

# Filter by assignee
fabric list --assignee @alice

# Output as JSON
fabric list --format json
```

### 4. View Task Details

```bash
fabric show task-abc123

# Include event history
fabric show task-abc123 --events
```

### 5. Update Tasks

```bash
# Update title
fabric update task-abc123 --title "New title"

# Update priority
fabric update task-abc123 --priority p0

# Assign to yourself
fabric claim task-abc123

# Assign to someone else
fabric assign task-abc123 @bob

# Unassign
fabric free task-abc123
```

### 6. Complete Tasks

```bash
# Mark as done
fabric complete task-abc123

# With resolution
fabric complete task-abc123 --resolution wontfix
```

Resolutions: `done`, `wontfix`, `duplicate`, `obsolete`

### 7. Interactive Shell

For rapid task management:

```bash
fabric shell
```

Commands work the same but without the `fabric` prefix:
```
> add "Quick task" -p p2
> list
> complete task-xyz
> quit
```

## Git Integration

Fabric events are regular files - commit them with your code:

```bash
git add .fabric/events/
git commit -m "Add authentication task"
```

Tasks follow branches. When you merge, events merge cleanly (append-only JSONL).

## Next Steps

- See [CLI Guide](CLI_GUIDE.md) for complete command reference
- Run `fabric --help` for all options
