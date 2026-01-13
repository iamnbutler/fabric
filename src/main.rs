use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

// =============================================================================
// Event Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub v: u32,
    pub op: Operation,
    pub id: String,
    pub ts: DateTime<Utc>,
    pub by: String,
    pub branch: String,
    pub d: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Update,
    Assign,
    Comment,
    Link,
    Unlink,
    Complete,
    Reopen,
    Archive,
}

// =============================================================================
// Task State
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub created: DateTime<Utc>,
    pub created_by: String,
    pub created_branch: String,
    pub updated: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Open,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub ts: DateTime<Utc>,
    pub by: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
}

// =============================================================================
// Index Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Index {
    pub tasks: HashMap<String, TaskIndex>,
    pub rebuilt: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIndex {
    pub status: TaskStatus,
    pub created: String,
    pub updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<String>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<String>,
}

// =============================================================================
// State (materialized view)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub tasks: HashMap<String, Task>,
    pub rebuilt: DateTime<Utc>,
}

// =============================================================================
// Fabric Context
// =============================================================================

pub struct FabricContext {
    pub root: PathBuf,
    pub events_dir: PathBuf,
    pub archive_dir: PathBuf,
}

impl FabricContext {
    pub fn discover() -> Result<Self> {
        let mut current = std::env::current_dir()?;
        loop {
            let fabric_dir = current.join(".fabric");
            if fabric_dir.is_dir() {
                return Ok(Self {
                    root: fabric_dir.clone(),
                    events_dir: fabric_dir.join("events"),
                    archive_dir: fabric_dir.join("archive"),
                });
            }
            if !current.pop() {
                return Err(anyhow!(
                    "Not in a fabric directory. Run 'fabric init' to create one."
                ));
            }
        }
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join(".index.json")
    }

    pub fn state_path(&self) -> PathBuf {
        self.root.join(".state.json")
    }

    pub fn get_event_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if self.events_dir.is_dir() {
            for entry in fs::read_dir(&self.events_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "jsonl") {
                    files.push(path);
                }
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn get_archive_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if self.archive_dir.is_dir() {
            for entry in fs::read_dir(&self.archive_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "jsonl") {
                    files.push(path);
                }
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn parse_events_from_file(&self, path: &Path) -> Result<Vec<Event>> {
        let file = File::open(path).with_context(|| format!("Failed to open {:?}", path))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: Event = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse line {} in {:?}", line_num + 1, path))?;
            events.push(event);
        }
        Ok(events)
    }
}

// =============================================================================
// State Materialization
// =============================================================================

pub fn materialize(ctx: &FabricContext) -> Result<State> {
    let mut tasks: HashMap<String, Task> = HashMap::new();

    // First process archive files
    for file in ctx.get_archive_files()? {
        let events = ctx.parse_events_from_file(&file)?;
        apply_events(&mut tasks, events);
    }

    // Then process event files (in chronological order)
    for file in ctx.get_event_files()? {
        let events = ctx.parse_events_from_file(&file)?;
        apply_events(&mut tasks, events);
    }

    Ok(State {
        tasks,
        rebuilt: Utc::now(),
    })
}

fn apply_events(tasks: &mut HashMap<String, Task>, events: Vec<Event>) {
    for event in events {
        apply_event(tasks, event);
    }
}

fn apply_event(tasks: &mut HashMap<String, Task>, event: Event) {
    match event.op {
        Operation::Create => {
            let d = &event.d;
            let task = Task {
                id: event.id.clone(),
                title: d.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: d.get("description").and_then(|v| v.as_str()).map(String::from),
                status: TaskStatus::Open,
                priority: d.get("priority").and_then(|v| v.as_str()).map(String::from),
                tags: d.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                assignee: d.get("assignee").and_then(|v| v.as_str()).map(String::from),
                created: event.ts,
                created_by: event.by.clone(),
                created_branch: event.branch.clone(),
                updated: event.ts,
                completed: None,
                resolution: None,
                parent: d.get("parent").and_then(|v| v.as_str()).map(String::from),
                blocks: d.get("blocks")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                blocked_by: d.get("blocked_by")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                comments: Vec::new(),
                archived: None,
            };
            tasks.insert(event.id, task);
        }
        Operation::Update => {
            if let Some(task) = tasks.get_mut(&event.id) {
                let d = &event.d;
                if let Some(title) = d.get("title").and_then(|v| v.as_str()) {
                    task.title = title.to_string();
                }
                if let Some(desc) = d.get("description").and_then(|v| v.as_str()) {
                    task.description = Some(desc.to_string());
                }
                if let Some(priority) = d.get("priority").and_then(|v| v.as_str()) {
                    task.priority = Some(priority.to_string());
                }
                if let Some(tags) = d.get("tags").and_then(|v| v.as_array()) {
                    task.tags = tags.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                }
                task.updated = event.ts;
            }
        }
        Operation::Assign => {
            if let Some(task) = tasks.get_mut(&event.id) {
                task.assignee = event.d.get("to").and_then(|v| {
                    if v.is_null() { None } else { v.as_str().map(String::from) }
                });
                task.updated = event.ts;
            }
        }
        Operation::Comment => {
            if let Some(task) = tasks.get_mut(&event.id) {
                let d = &event.d;
                task.comments.push(Comment {
                    ts: event.ts,
                    by: event.by,
                    body: d.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    r#ref: d.get("ref").and_then(|v| v.as_str()).map(String::from),
                });
                task.updated = event.ts;
            }
        }
        Operation::Link => {
            if let Some(task) = tasks.get_mut(&event.id) {
                let d = &event.d;
                if let (Some(rel), Some(target)) = (
                    d.get("rel").and_then(|v| v.as_str()),
                    d.get("target").and_then(|v| v.as_str()),
                ) {
                    match rel {
                        "blocks" => {
                            if !task.blocks.contains(&target.to_string()) {
                                task.blocks.push(target.to_string());
                            }
                        }
                        "blocked_by" => {
                            if !task.blocked_by.contains(&target.to_string()) {
                                task.blocked_by.push(target.to_string());
                            }
                        }
                        "parent" => task.parent = Some(target.to_string()),
                        _ => {}
                    }
                }
                task.updated = event.ts;
            }
        }
        Operation::Unlink => {
            if let Some(task) = tasks.get_mut(&event.id) {
                let d = &event.d;
                if let (Some(rel), Some(target)) = (
                    d.get("rel").and_then(|v| v.as_str()),
                    d.get("target").and_then(|v| v.as_str()),
                ) {
                    match rel {
                        "blocks" => task.blocks.retain(|x| x != target),
                        "blocked_by" => task.blocked_by.retain(|x| x != target),
                        "parent" => {
                            if task.parent.as_deref() == Some(target) {
                                task.parent = None;
                            }
                        }
                        _ => {}
                    }
                }
                task.updated = event.ts;
            }
        }
        Operation::Complete => {
            if let Some(task) = tasks.get_mut(&event.id) {
                task.status = TaskStatus::Complete;
                task.completed = Some(event.ts);
                task.resolution = event.d
                    .get("resolution")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or(Some("done".to_string()));
                task.updated = event.ts;
            }
        }
        Operation::Reopen => {
            if let Some(task) = tasks.get_mut(&event.id) {
                task.status = TaskStatus::Open;
                task.completed = None;
                task.resolution = None;
                task.updated = event.ts;
            }
        }
        Operation::Archive => {
            if let Some(task) = tasks.get_mut(&event.id) {
                task.archived = event.d.get("ref").and_then(|v| v.as_str()).map(String::from);
                task.updated = event.ts;
            }
        }
    }
}

// =============================================================================
// Index Building
// =============================================================================

pub fn build_index(ctx: &FabricContext) -> Result<Index> {
    let mut task_files: HashMap<String, HashSet<String>> = HashMap::new();
    let mut task_info: HashMap<String, (TaskStatus, String, String, Option<String>, Option<String>)> = HashMap::new();

    // Track which files contain events for each task
    for file in ctx.get_event_files()? {
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        let events = ctx.parse_events_from_file(&file)?;
        for event in events {
            task_files
                .entry(event.id.clone())
                .or_default()
                .insert(filename.clone());

            let date = event.ts.format("%Y-%m-%d").to_string();

            match event.op {
                Operation::Create => {
                    task_info.insert(
                        event.id.clone(),
                        (TaskStatus::Open, date.clone(), date, None, None),
                    );
                }
                Operation::Complete => {
                    if let Some(info) = task_info.get_mut(&event.id) {
                        info.0 = TaskStatus::Complete;
                        info.2 = date.clone();
                        info.3 = Some(date);
                    }
                }
                Operation::Reopen => {
                    if let Some(info) = task_info.get_mut(&event.id) {
                        info.0 = TaskStatus::Open;
                        info.2 = date;
                        info.3 = None;
                    }
                }
                Operation::Archive => {
                    if let Some(info) = task_info.get_mut(&event.id) {
                        info.2 = date;
                        info.4 = event.d.get("ref").and_then(|v| v.as_str()).map(String::from);
                    }
                }
                _ => {
                    if let Some(info) = task_info.get_mut(&event.id) {
                        info.2 = date;
                    }
                }
            }
        }
    }

    let mut tasks = HashMap::new();
    for (id, info) in task_info {
        let files: Vec<String> = task_files
            .get(&id)
            .map(|s| {
                let mut v: Vec<_> = s.iter().cloned().collect();
                v.sort();
                v
            })
            .unwrap_or_default();

        tasks.insert(
            id,
            TaskIndex {
                status: info.0,
                created: info.1,
                updated: info.2,
                completed: info.3,
                files,
                archived: info.4,
            },
        );
    }

    Ok(Index {
        tasks,
        rebuilt: Utc::now(),
    })
}

// =============================================================================
// Archive
// =============================================================================

pub fn archive_tasks(ctx: &FabricContext, days: u32, dry_run: bool) -> Result<Vec<String>> {
    let state = materialize(ctx)?;
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);

    let mut to_archive: Vec<&Task> = state
        .tasks
        .values()
        .filter(|t| {
            t.status == TaskStatus::Complete
                && t.completed.map_or(false, |c| c < cutoff)
                && t.archived.is_none()
        })
        .collect();

    to_archive.sort_by_key(|t| t.completed);

    if to_archive.is_empty() {
        println!("No tasks to archive.");
        return Ok(Vec::new());
    }

    let archived_ids: Vec<String> = to_archive.iter().map(|t| t.id.clone()).collect();

    if dry_run {
        println!("Would archive {} tasks:", to_archive.len());
        for task in &to_archive {
            println!("  {} - {}", task.id, task.title);
        }
        return Ok(archived_ids);
    }

    // Group tasks by completion month
    let mut by_month: BTreeMap<String, Vec<&Task>> = BTreeMap::new();
    for task in &to_archive {
        if let Some(completed) = task.completed {
            let month = completed.format("%Y-%m").to_string();
            by_month.entry(month).or_default().push(task);
        }
    }

    // Create archive directory if needed
    fs::create_dir_all(&ctx.archive_dir)?;

    // Collect all events for archived tasks and write to monthly files
    let all_events = collect_all_events(ctx)?;

    for (month, tasks) in &by_month {
        let archive_file = ctx.archive_dir.join(format!("{}.jsonl", month));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&archive_file)?;
        let mut writer = BufWriter::new(file);

        for task in tasks {
            // Write all events for this task to the archive
            if let Some(events) = all_events.get(&task.id) {
                for event in events {
                    let json = serde_json::to_string(event)?;
                    writeln!(writer, "{}", json)?;
                }
            }
        }
        writer.flush()?;
    }

    // Emit archive events to today's event file
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let event_file = ctx.events_dir.join(format!("{}.jsonl", today));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&event_file)?;
    let mut writer = BufWriter::new(file);

    let branch = get_current_branch()?;

    for task in &to_archive {
        if let Some(completed) = task.completed {
            let month = completed.format("%Y-%m").to_string();
            let archive_event = Event {
                v: 1,
                op: Operation::Archive,
                id: task.id.clone(),
                ts: Utc::now(),
                by: "@fabric".to_string(),
                branch: branch.clone(),
                d: serde_json::json!({ "ref": month }),
            };
            let json = serde_json::to_string(&archive_event)?;
            writeln!(writer, "{}", json)?;
        }
    }
    writer.flush()?;

    println!("Archived {} tasks.", to_archive.len());
    for (month, tasks) in &by_month {
        println!("  {} tasks to archive/{}.jsonl", tasks.len(), month);
    }

    Ok(archived_ids)
}

fn collect_all_events(ctx: &FabricContext) -> Result<HashMap<String, Vec<Event>>> {
    let mut events_by_task: HashMap<String, Vec<Event>> = HashMap::new();

    for file in ctx.get_event_files()? {
        let events = ctx.parse_events_from_file(&file)?;
        for event in events {
            events_by_task
                .entry(event.id.clone())
                .or_default()
                .push(event);
        }
    }

    Ok(events_by_task)
}

fn get_current_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("main".to_string())
    }
}

// =============================================================================
// Validation
// =============================================================================

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate(ctx: &FabricContext, strict: bool) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut created_ids: HashSet<String> = HashSet::new();

    // Validate event files
    for file in ctx.get_event_files()? {
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        validate_event_file(&file, &filename, &mut errors, &mut warnings, &mut seen_ids, &mut created_ids)?;
    }

    // Validate archive files
    for file in ctx.get_archive_files()? {
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        validate_event_file(&file, &filename, &mut errors, &mut warnings, &mut seen_ids, &mut created_ids)?;
    }

    // Check for orphaned references
    let state = materialize(ctx)?;
    for task in state.tasks.values() {
        for blocked_by in &task.blocked_by {
            if !state.tasks.contains_key(blocked_by) {
                warnings.push(format!(
                    "Task {} references non-existent blocked_by: {}",
                    task.id, blocked_by
                ));
            }
        }
        for blocks in &task.blocks {
            if !state.tasks.contains_key(blocks) {
                warnings.push(format!(
                    "Task {} references non-existent blocks: {}",
                    task.id, blocks
                ));
            }
        }
        if let Some(parent) = &task.parent {
            if !state.tasks.contains_key(parent) {
                warnings.push(format!(
                    "Task {} references non-existent parent: {}",
                    task.id, parent
                ));
            }
        }
    }

    let result = ValidationResult { errors, warnings };

    // Print results
    if result.errors.is_empty() && result.warnings.is_empty() {
        println!("Validation passed. No issues found.");
    } else {
        if !result.errors.is_empty() {
            println!("Errors ({}):", result.errors.len());
            for error in &result.errors {
                println!("  ERROR: {}", error);
            }
        }
        if !result.warnings.is_empty() {
            println!("Warnings ({}):", result.warnings.len());
            for warning in &result.warnings {
                println!("  WARN: {}", warning);
            }
        }

        if strict && !result.errors.is_empty() {
            return Err(anyhow!("Validation failed with {} errors", result.errors.len()));
        }
        if strict && !result.warnings.is_empty() {
            return Err(anyhow!("Validation failed with {} warnings (--strict mode)", result.warnings.len()));
        }
    }

    Ok(result)
}

fn validate_event_file(
    path: &Path,
    filename: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    _seen_ids: &mut HashSet<String>,
    created_ids: &mut HashSet<String>,
) -> Result<()> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            errors.push(format!("Cannot open {}: {}", filename, e));
            return Ok(());
        }
    };
    let reader = BufReader::new(file);

    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                errors.push(format!("{}:{}: Read error: {}", filename, line_num + 1, e));
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let event: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}:{}: Invalid JSON: {}", filename, line_num + 1, e));
                continue;
            }
        };

        // Check required fields
        let required = ["v", "op", "id", "ts", "by", "branch", "d"];
        for field in required {
            if event.get(field).is_none() {
                errors.push(format!(
                    "{}:{}: Missing required field '{}'",
                    filename,
                    line_num + 1,
                    field
                ));
            }
        }

        // Check schema version
        if let Some(v) = event.get("v").and_then(|v| v.as_u64()) {
            if v != 1 {
                warnings.push(format!(
                    "{}:{}: Unknown schema version {}",
                    filename,
                    line_num + 1,
                    v
                ));
            }
        }

        // Track creates for orphan detection
        if let Some(op) = event.get("op").and_then(|v| v.as_str()) {
            if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                if op == "create" {
                    if created_ids.contains(id) {
                        warnings.push(format!(
                            "{}:{}: Duplicate create for task {}",
                            filename,
                            line_num + 1,
                            id
                        ));
                    }
                    created_ids.insert(id.to_string());
                } else if !created_ids.contains(id) {
                    warnings.push(format!(
                        "{}:{}: Event for task {} before create",
                        filename,
                        line_num + 1,
                        id
                    ));
                }
            }
        }

        // Validate timestamp format
        if let Some(ts) = event.get("ts").and_then(|v| v.as_str()) {
            if DateTime::parse_from_rfc3339(ts).is_err() {
                errors.push(format!(
                    "{}:{}: Invalid timestamp format: {}",
                    filename,
                    line_num + 1,
                    ts
                ));
            }
        }
    }

    Ok(())
}

// =============================================================================
// Rebuild
// =============================================================================

pub fn rebuild(ctx: &FabricContext) -> Result<()> {
    println!("Rebuilding index and state...");

    // Build and write index
    let index = build_index(ctx)?;
    let index_json = serde_json::to_string_pretty(&index)?;
    fs::write(ctx.index_path(), index_json)?;
    println!("  Wrote .index.json ({} tasks)", index.tasks.len());

    // Build and write state
    let state = materialize(ctx)?;
    let state_json = serde_json::to_string_pretty(&state)?;
    fs::write(ctx.state_path(), state_json)?;
    println!("  Wrote .state.json ({} tasks)", state.tasks.len());

    println!("Rebuild complete.");
    Ok(())
}

// =============================================================================
// Init
// =============================================================================

pub fn init() -> Result<()> {
    let fabric_dir = PathBuf::from(".fabric");

    if fabric_dir.exists() {
        return Err(anyhow!(".fabric directory already exists"));
    }

    fs::create_dir_all(fabric_dir.join("events"))?;
    fs::create_dir_all(fabric_dir.join("archive"))?;

    // Create .gitignore
    let gitignore = r#"# Derived files - rebuilt from events on checkout/merge
# These are caches for fast queries, not source of truth

# Task index: maps task_id → status, date range, file locations
.index.json

# Materialized state: current snapshot of all tasks
.state.json

# Any temporary files from tooling
*.tmp
*.bak
"#;
    fs::write(fabric_dir.join(".gitignore"), gitignore)?;

    println!("Created .fabric/");
    println!("  .fabric/events/     - Daily event logs");
    println!("  .fabric/archive/    - Monthly rollups");
    println!("  .fabric/.gitignore  - Ignores derived files");

    Ok(())
}

// =============================================================================
// Query Functions
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Table,
    Json,
    Ids,
}

pub fn list_tasks(
    ctx: &FabricContext,
    status_filter: Option<&str>,
    assignee: Option<&str>,
    tag: Option<&str>,
    priority: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let state = load_or_materialize_state(ctx)?;

    let mut tasks: Vec<&Task> = state
        .tasks
        .values()
        .filter(|t| {
            // Status filter
            let status_match = match status_filter {
                Some("open") => t.status == TaskStatus::Open,
                Some("complete") => t.status == TaskStatus::Complete,
                Some("all") | None => true,
                _ => true,
            };

            // Assignee filter
            let assignee_match = assignee
                .map(|a| t.assignee.as_deref() == Some(a))
                .unwrap_or(true);

            // Tag filter
            let tag_match = tag
                .map(|tg| t.tags.iter().any(|t| t == tg))
                .unwrap_or(true);

            // Priority filter
            let priority_match = priority
                .map(|p| t.priority.as_deref() == Some(p))
                .unwrap_or(true);

            status_match && assignee_match && tag_match && priority_match
        })
        .collect();

    // Sort by created date
    tasks.sort_by_key(|t| t.created);

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&tasks)?;
            println!("{}", json);
        }
        OutputFormat::Ids => {
            for task in &tasks {
                println!("{}", task.id);
            }
        }
        OutputFormat::Table => {
            if tasks.is_empty() {
                println!("No tasks found.");
                return Ok(());
            }

            println!(
                "{:<15} {:<10} {:<12} {}",
                "ID", "PRIORITY", "ASSIGNEE", "TITLE"
            );
            for task in &tasks {
                let priority = task.priority.as_deref().unwrap_or("-");
                let assignee = task.assignee.as_deref().unwrap_or("-");
                let title = if task.title.len() > 50 {
                    format!("{}...", &task.title[..47])
                } else {
                    task.title.clone()
                };
                println!("{:<15} {:<10} {:<12} {}", task.id, priority, assignee, title);
            }
        }
    }

    Ok(())
}

pub fn show_task(ctx: &FabricContext, id: &str, show_events: bool) -> Result<()> {
    let state = load_or_materialize_state(ctx)?;

    let task = state.tasks.get(id).ok_or_else(|| anyhow!("Task not found: {}", id))?;

    println!("ID:       {}", task.id);
    println!("Title:    {}", task.title);
    println!("Status:   {:?}", task.status);
    if let Some(p) = &task.priority {
        println!("Priority: {}", p);
    }
    if let Some(a) = &task.assignee {
        println!("Assignee: {}", a);
    }
    if !task.tags.is_empty() {
        println!("Tags:     {}", task.tags.join(", "));
    }
    if let Some(d) = &task.description {
        println!("Description:\n  {}", d.replace('\n', "\n  "));
    }
    println!("Created:  {} by {} on {}", task.created, task.created_by, task.created_branch);
    println!("Updated:  {}", task.updated);
    if let Some(c) = task.completed {
        println!("Completed: {} ({})", c, task.resolution.as_deref().unwrap_or("done"));
    }
    if let Some(a) = &task.archived {
        println!("Archived: {}", a);
    }
    if let Some(p) = &task.parent {
        println!("Parent:   {}", p);
    }
    if !task.blocks.is_empty() {
        println!("Blocks:   {}", task.blocks.join(", "));
    }
    if !task.blocked_by.is_empty() {
        println!("Blocked by: {}", task.blocked_by.join(", "));
    }

    if !task.comments.is_empty() {
        println!("\nComments:");
        for comment in &task.comments {
            println!("  [{} - {}]", comment.ts, comment.by);
            println!("  {}", comment.body.replace('\n', "\n  "));
            if let Some(r) = &comment.r#ref {
                println!("  ref: {}", r);
            }
            println!();
        }
    }

    if show_events {
        println!("\nEvent History:");
        let all_events = collect_all_events(ctx)?;
        if let Some(events) = all_events.get(id) {
            for event in events {
                println!("  {} {} by {} on {}", event.ts, event.op.to_string(), event.by, event.branch);
            }
        }
    }

    Ok(())
}

fn load_or_materialize_state(ctx: &FabricContext) -> Result<State> {
    let state_path = ctx.state_path();
    if state_path.exists() {
        let content = fs::read_to_string(&state_path)?;
        let state: State = serde_json::from_str(&content)?;
        Ok(state)
    } else {
        materialize(ctx)
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Create => write!(f, "create"),
            Operation::Update => write!(f, "update"),
            Operation::Assign => write!(f, "assign"),
            Operation::Comment => write!(f, "comment"),
            Operation::Link => write!(f, "link"),
            Operation::Unlink => write!(f, "unlink"),
            Operation::Complete => write!(f, "complete"),
            Operation::Reopen => write!(f, "reopen"),
            Operation::Archive => write!(f, "archive"),
        }
    }
}

// =============================================================================
// CLI
// =============================================================================

#[derive(Parser)]
#[command(name = "fabric")]
#[command(about = "Git-native task management system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .fabric/ directory structure
    Init,
    /// List tasks with optional filtering
    List {
        /// Status filter: open, complete, or all (default: open)
        #[arg(short, long, default_value = "open")]
        status: String,
        /// Filter by assignee
        #[arg(short, long)]
        assignee: Option<String>,
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Filter by priority
        #[arg(short, long)]
        priority: Option<String>,
        /// Output format: table, json, or ids
        #[arg(short, long, default_value = "table")]
        format: String,
    },
    /// Show details of a specific task
    Show {
        /// Task ID to show
        id: String,
        /// Show raw event history
        #[arg(long)]
        events: bool,
    },
    /// Rebuild .index.json and .state.json from events
    Rebuild,
    /// Archive completed tasks older than N days
    Archive {
        /// Days after completion to archive (default: 30)
        #[arg(short, long, default_value = "30")]
        days: u32,
        /// Show what would be archived without doing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate event files for correctness
    Validate {
        /// Fail on warnings too
        #[arg(long)]
        strict: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init(),
        Commands::List {
            status,
            assignee,
            tag,
            priority,
            format,
        } => {
            let ctx = FabricContext::discover()?;
            let fmt = match format.as_str() {
                "json" => OutputFormat::Json,
                "ids" => OutputFormat::Ids,
                _ => OutputFormat::Table,
            };
            list_tasks(
                &ctx,
                Some(&status),
                assignee.as_deref(),
                tag.as_deref(),
                priority.as_deref(),
                fmt,
            )
        }
        Commands::Show { id, events } => {
            let ctx = FabricContext::discover()?;
            show_task(&ctx, &id, events)
        }
        Commands::Rebuild => {
            let ctx = FabricContext::discover()?;
            rebuild(&ctx)
        }
        Commands::Archive { days, dry_run } => {
            let ctx = FabricContext::discover()?;
            archive_tasks(&ctx, days, dry_run)?;
            Ok(())
        }
        Commands::Validate { strict } => {
            let ctx = FabricContext::discover()?;
            validate(&ctx, strict)?;
            Ok(())
        }
    }
}
