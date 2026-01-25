use anyhow::Result;
use spool::context::SpoolContext;
use spool::state::{load_or_materialize_state, Task, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    TaskList,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StatusFilter {
    Open,
    Complete,
    All,
}

impl StatusFilter {
    pub fn label(&self) -> &'static str {
        match self {
            StatusFilter::Open => "Open",
            StatusFilter::Complete => "Complete",
            StatusFilter::All => "All",
        }
    }
}

pub struct App {
    pub tasks: Vec<Task>,
    pub selected: usize,
    pub focus: Focus,
    pub show_detail: bool,
    pub status_filter: StatusFilter,
    #[allow(dead_code)]
    ctx: SpoolContext,
}

impl App {
    pub fn new() -> Result<Self> {
        let ctx = SpoolContext::discover()?;
        let state = load_or_materialize_state(&ctx)?;

        let mut tasks: Vec<Task> = state
            .tasks
            .into_values()
            .filter(|t| t.status == TaskStatus::Open)
            .collect();

        // Sort by priority, then by created date
        tasks.sort_by(|a, b| {
            let pa = a.priority.as_deref().unwrap_or("p3");
            let pb = b.priority.as_deref().unwrap_or("p3");
            pa.cmp(pb).then_with(|| a.created.cmp(&b.created))
        });

        Ok(Self {
            tasks,
            selected: 0,
            focus: Focus::TaskList,
            show_detail: false,
            status_filter: StatusFilter::Open,
            ctx,
        })
    }

    #[allow(dead_code)]
    pub fn reload_tasks(&mut self) -> Result<()> {
        let state = load_or_materialize_state(&self.ctx)?;

        let mut tasks: Vec<Task> = state
            .tasks
            .into_values()
            .filter(|t| match self.status_filter {
                StatusFilter::Open => t.status == TaskStatus::Open,
                StatusFilter::Complete => t.status == TaskStatus::Complete,
                StatusFilter::All => true,
            })
            .collect();

        tasks.sort_by(|a, b| {
            let pa = a.priority.as_deref().unwrap_or("p3");
            let pb = b.priority.as_deref().unwrap_or("p3");
            pa.cmp(pb).then_with(|| a.created.cmp(&b.created))
        });

        self.tasks = tasks;
        if self.selected >= self.tasks.len() && !self.tasks.is_empty() {
            self.selected = self.tasks.len() - 1;
        }

        Ok(())
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected)
    }

    pub fn next_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected = (self.selected + 1).min(self.tasks.len() - 1);
        }
    }

    pub fn previous_task(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn first_task(&mut self) {
        self.selected = 0;
    }

    pub fn last_task(&mut self) {
        if !self.tasks.is_empty() {
            self.selected = self.tasks.len() - 1;
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::TaskList => Focus::Detail,
            Focus::Detail => Focus::TaskList,
        };
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }
}
