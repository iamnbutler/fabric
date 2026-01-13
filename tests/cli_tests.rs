use clap::Parser;
use fabric::cli::{Cli, Commands, OutputFormat};

#[test]
fn test_output_format_from_str() {
    assert_eq!(OutputFormat::from_str("table"), OutputFormat::Table);
    assert_eq!(OutputFormat::from_str("json"), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str("ids"), OutputFormat::Ids);

    // Unknown format defaults to Table
    assert_eq!(OutputFormat::from_str("unknown"), OutputFormat::Table);
    assert_eq!(OutputFormat::from_str(""), OutputFormat::Table);
}

#[test]
fn test_cli_parse_init() {
    let cli = Cli::parse_from(["fabric", "init"]);
    assert!(matches!(cli.command, Commands::Init));
}

#[test]
fn test_cli_parse_list_defaults() {
    let cli = Cli::parse_from(["fabric", "list"]);

    if let Commands::List {
        status,
        assignee,
        tag,
        priority,
        format,
    } = cli.command
    {
        assert_eq!(status, "open");
        assert!(assignee.is_none());
        assert!(tag.is_none());
        assert!(priority.is_none());
        assert_eq!(format, "table");
    } else {
        panic!("Expected List command");
    }
}

#[test]
fn test_cli_parse_list_with_filters() {
    let cli = Cli::parse_from([
        "fabric",
        "list",
        "--status",
        "complete",
        "--assignee",
        "dev1",
        "--tag",
        "bug",
        "--priority",
        "p1",
        "--format",
        "json",
    ]);

    if let Commands::List {
        status,
        assignee,
        tag,
        priority,
        format,
    } = cli.command
    {
        assert_eq!(status, "complete");
        assert_eq!(assignee.as_deref(), Some("dev1"));
        assert_eq!(tag.as_deref(), Some("bug"));
        assert_eq!(priority.as_deref(), Some("p1"));
        assert_eq!(format, "json");
    } else {
        panic!("Expected List command");
    }
}

#[test]
fn test_cli_parse_list_short_flags() {
    let cli = Cli::parse_from([
        "fabric", "list", "-s", "all", "-a", "user", "-t", "feature", "-p", "p2", "-f", "ids",
    ]);

    if let Commands::List {
        status,
        assignee,
        tag,
        priority,
        format,
    } = cli.command
    {
        assert_eq!(status, "all");
        assert_eq!(assignee.as_deref(), Some("user"));
        assert_eq!(tag.as_deref(), Some("feature"));
        assert_eq!(priority.as_deref(), Some("p2"));
        assert_eq!(format, "ids");
    } else {
        panic!("Expected List command");
    }
}

#[test]
fn test_cli_parse_show() {
    let cli = Cli::parse_from(["fabric", "show", "task-123"]);

    if let Commands::Show { id, events } = cli.command {
        assert_eq!(id, "task-123");
        assert!(!events);
    } else {
        panic!("Expected Show command");
    }
}

#[test]
fn test_cli_parse_show_with_events() {
    let cli = Cli::parse_from(["fabric", "show", "task-456", "--events"]);

    if let Commands::Show { id, events } = cli.command {
        assert_eq!(id, "task-456");
        assert!(events);
    } else {
        panic!("Expected Show command");
    }
}

#[test]
fn test_cli_parse_rebuild() {
    let cli = Cli::parse_from(["fabric", "rebuild"]);
    assert!(matches!(cli.command, Commands::Rebuild));
}

#[test]
fn test_cli_parse_archive_defaults() {
    let cli = Cli::parse_from(["fabric", "archive"]);

    if let Commands::Archive { days, dry_run } = cli.command {
        assert_eq!(days, 30);
        assert!(!dry_run);
    } else {
        panic!("Expected Archive command");
    }
}

#[test]
fn test_cli_parse_archive_with_options() {
    let cli = Cli::parse_from(["fabric", "archive", "--days", "60", "--dry-run"]);

    if let Commands::Archive { days, dry_run } = cli.command {
        assert_eq!(days, 60);
        assert!(dry_run);
    } else {
        panic!("Expected Archive command");
    }
}

#[test]
fn test_cli_parse_archive_short_flag() {
    let cli = Cli::parse_from(["fabric", "archive", "-d", "7"]);

    if let Commands::Archive { days, dry_run } = cli.command {
        assert_eq!(days, 7);
        assert!(!dry_run);
    } else {
        panic!("Expected Archive command");
    }
}

#[test]
fn test_cli_parse_validate() {
    let cli = Cli::parse_from(["fabric", "validate"]);

    if let Commands::Validate { strict } = cli.command {
        assert!(!strict);
    } else {
        panic!("Expected Validate command");
    }
}

#[test]
fn test_cli_parse_validate_strict() {
    let cli = Cli::parse_from(["fabric", "validate", "--strict"]);

    if let Commands::Validate { strict } = cli.command {
        assert!(strict);
    } else {
        panic!("Expected Validate command");
    }
}

#[test]
fn test_output_format_equality() {
    assert!(OutputFormat::Table == OutputFormat::Table);
    assert!(OutputFormat::Json == OutputFormat::Json);
    assert!(OutputFormat::Ids == OutputFormat::Ids);
    assert!(OutputFormat::Table != OutputFormat::Json);
}

#[test]
fn test_output_format_clone() {
    let format = OutputFormat::Json;
    let cloned = format;
    assert_eq!(format, cloned);
}
