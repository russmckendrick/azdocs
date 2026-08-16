use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// Audit and document an Azure estate using Azure Resource Graph.
///
/// Collects resource data with read-only credentials into a local SQLite
/// database, then exports reports (Markdown, HTML, CSV, XLSX) and diagrams
/// (draw.io, Mermaid) without further network access.
#[derive(Debug, Parser)]
#[command(name = "azdocs", version, propagate_version = true)]
pub struct Cli {
    /// Path to the config file (default: ./azdocs.toml, then platform config dir)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Path to the SQLite database (overrides config)
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<PathBuf>,

    /// Increase log verbosity (-v info, -vv debug)
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    /// Disable coloured output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create an azdocs.toml config file interactively
    Init {
        /// Overwrite an existing config file
        #[arg(long)]
        force: bool,
        /// Fail instead of prompting when values are missing
        #[arg(long)]
        non_interactive: bool,
    },

    /// Validate config, acquire a token, and run a probe query
    Check,

    /// Run the query pack and store the results as a new snapshot
    Collect(CollectArgs),

    /// List, inspect, diff, and prune stored snapshots
    #[command(subcommand)]
    Snapshots(SnapshotsCommand),

    /// Export reports from a stored snapshot
    Report(ReportArgs),

    /// Export diagrams from a stored snapshot
    Diagram(DiagramArgs),

    /// List, inspect, and run individual queries
    #[command(subcommand)]
    Query(QueryCommand),

    /// Browse a stored snapshot in an interactive TUI
    Browse {
        /// Snapshot id or "latest"
        #[arg(long, default_value = "latest")]
        snapshot: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Debug, Args)]
pub struct CollectArgs {
    /// Subscription ids to collect (default: all visible to the credential)
    #[arg(long, value_delimiter = ',', value_name = "ID")]
    pub subscriptions: Vec<String>,

    /// Only run queries in these categories
    #[arg(long, value_delimiter = ',', value_name = "CATEGORY")]
    pub categories: Vec<String>,

    /// Only run these named queries
    #[arg(long, value_delimiter = ',', value_name = "NAME")]
    pub queries: Vec<String>,

    /// Skip these named queries
    #[arg(long, value_delimiter = ',', value_name = "NAME")]
    pub skip_queries: Vec<String>,

    /// Free-text note stored with the snapshot
    #[arg(long)]
    pub notes: Option<String>,

    /// Maximum concurrent queries (overrides config)
    #[arg(long)]
    pub concurrency: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum SnapshotsCommand {
    /// List stored snapshots
    List,
    /// Show a snapshot's summary and query run details
    Show {
        /// Snapshot id or "latest"
        #[arg(default_value = "latest")]
        snapshot: String,
    },
    /// Compare two snapshots: added, removed, and changed resources
    Diff {
        /// Older snapshot id
        a: String,
        /// Newer snapshot id or "latest"
        b: String,
        #[arg(long, value_enum, default_value_t = DiffFormat::Table)]
        format: DiffFormat,
    },
    /// Delete old snapshots
    Prune {
        /// Keep the most recent N snapshots
        #[arg(long, value_name = "N")]
        keep: Option<usize>,
        /// Delete snapshots older than this many days
        #[arg(long, value_name = "DAYS")]
        older_than: Option<u32>,
        /// Do not ask for confirmation
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiffFormat {
    Table,
    Md,
    Json,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Snapshot id or "latest"
    #[arg(long, default_value = "latest")]
    pub snapshot: String,

    /// Report format to generate
    #[arg(long, value_enum, default_value_t = ReportFormat::Md)]
    pub format: ReportFormat,

    /// Output directory (default: ./output)
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    Md,
    Html,
    Csv,
    Xlsx,
    All,
}

#[derive(Debug, Args)]
pub struct DiagramArgs {
    /// Snapshot id or "latest"
    #[arg(long, default_value = "latest")]
    pub snapshot: String,

    /// Diagram type to generate
    #[arg(long = "type", value_enum)]
    pub diagram_type: DiagramType,

    /// Diagram format to generate
    #[arg(long, value_enum, default_value_t = DiagramFormat::Drawio)]
    pub format: DiagramFormat,

    /// Restrict to one subscription id
    #[arg(long, value_name = "ID")]
    pub subscription: Option<String>,

    /// Restrict to one resource group name
    #[arg(long, value_name = "NAME")]
    pub resource_group: Option<String>,

    /// Output file path (default: ./output/azdocs-<type>.<ext>)
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagramType {
    Hierarchy,
    Resources,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagramFormat {
    Drawio,
    Mermaid,
    Both,
}

#[derive(Debug, Subcommand)]
pub enum QueryCommand {
    /// List available queries (built-in and user-defined)
    List {
        /// Only list queries in this category
        #[arg(long)]
        category: Option<String>,
    },
    /// Print a query's KQL
    Show {
        /// Query name
        name: String,
    },
    /// Run one query live against Azure Resource Graph and print the results
    Run {
        /// Query name, path to a .toml/.kql file, or "-" for stdin
        query: String,
        #[arg(long, value_enum, default_value_t = QueryOutputFormat::Table)]
        format: QueryOutputFormat,
        /// Subscription ids to scope the query to
        #[arg(long, value_delimiter = ',', value_name = "ID")]
        subscriptions: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum QueryOutputFormat {
    Table,
    Json,
    Csv,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_asserts_valid_definition() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn collect_parses_comma_separated_subscriptions() {
        let cli = Cli::parse_from(["azdocs", "collect", "--subscriptions", "a,b"]);
        let Command::Collect(args) = cli.command else {
            panic!("expected collect subcommand");
        };
        assert_eq!(args.subscriptions, vec!["a", "b"]);
    }

    #[test]
    fn diagram_requires_type() {
        let result = Cli::try_parse_from(["azdocs", "diagram"]);
        assert!(result.is_err());
    }
}
