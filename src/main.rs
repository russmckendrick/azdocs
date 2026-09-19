use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use azdocs::cli::{
    Cli, CollectArgs, Command, ConfigCommand, DiagramArgs, OutputFormat, QueryCommand,
    QueryOutputFormat, ReportArgs, SnapshotsCommand,
};
use azdocs::commands;
use azdocs::config::{Config, ConfigDocument};
use azdocs::labels::{DEFAULT_LABELS, LabelPack, Labels};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let color = color_enabled(cli.no_color, std::env::var_os("NO_COLOR"));
    init_tracing(cli.verbose, color);

    match cli.command {
        Command::Init {
            force,
            non_interactive,
            secret_env,
        } => run_init(
            cli.config.as_deref(),
            commands::init::InitOptions {
                force,
                non_interactive,
                secret_env,
            },
        ),
        Command::Config(command) => {
            run_config(&command, cli.config.as_deref(), cli.tenant.as_deref())
        }
        Command::Check { format } => {
            Box::pin(run_check(
                cli.config.as_deref(),
                cli.tenant.as_deref(),
                format,
            ))
            .await
        }
        Command::Query(QueryCommand::Run {
            query,
            format,
            subscriptions,
        }) => {
            Box::pin(run_query(
                cli.config.as_deref(),
                cli.tenant.as_deref(),
                query,
                format,
                subscriptions,
            ))
            .await
        }
        Command::Query(QueryCommand::List { category, format }) => {
            run_query_list(category.as_deref(), format)
        }
        Command::Query(QueryCommand::Show { name }) => commands::query::show(&name),
        Command::Collect(args) => {
            Box::pin(run_collect(
                cli.config.as_deref(),
                cli.db.as_deref(),
                cli.tenant.as_deref(),
                args,
            ))
            .await
        }
        Command::Diagram(args) => run_diagram(
            cli.config.as_deref(),
            cli.db.as_deref(),
            cli.tenant.as_deref(),
            args,
        ),
        Command::Report(args) => run_report(
            cli.config.as_deref(),
            cli.db.as_deref(),
            cli.tenant.as_deref(),
            args,
        ),
        Command::Browse { snapshot } => run_browse(
            cli.config.as_deref(),
            cli.db.as_deref(),
            cli.tenant.as_deref(),
            snapshot,
        ),
        Command::Snapshots(command) => run_snapshots(
            cli.config.as_deref(),
            cli.db.as_deref(),
            cli.tenant.as_deref(),
            command,
        ),
        Command::Completions { shell } => run_completions(shell),
    }
}

fn run_init(config: Option<&std::path::Path>, options: commands::init::InitOptions) -> Result<()> {
    commands::init::run(config, &options, &default_labels()?)
}

fn run_config(
    command: &ConfigCommand,
    config: Option<&std::path::Path>,
    tenant: Option<&str>,
) -> Result<()> {
    if matches!(command, ConfigCommand::Show) {
        commands::config::show(config, tenant)
    } else {
        commands::config::run(command, config, tenant, &default_labels()?)
    }
}

async fn run_check(
    config: Option<&std::path::Path>,
    tenant: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let config = ConfigDocument::load(config)?.resolve(tenant)?;
    let labels = azdocs::labels::resolve(&config.branding)?;
    commands::check::run(&config, format, &labels).await
}

async fn run_query(
    config: Option<&std::path::Path>,
    tenant: Option<&str>,
    query: String,
    format: QueryOutputFormat,
    subscriptions: Vec<String>,
) -> Result<()> {
    let config = ConfigDocument::load(config)?.resolve(tenant)?;
    let labels = azdocs::labels::resolve(&config.branding)?;
    commands::query::run(&config, &query, format, &subscriptions, &labels).await
}

fn run_query_list(category: Option<&str>, format: OutputFormat) -> Result<()> {
    commands::query::list(category, format, &default_labels()?)
}

async fn run_collect(
    config: Option<&std::path::Path>,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    args: CollectArgs,
) -> Result<()> {
    let config = ConfigDocument::load(config)?.resolve(tenant)?;
    let labels = azdocs::labels::resolve(&config.branding)?;
    if args.dry_run {
        // Never opens the store or the network: the whole point.
        return commands::collect::dry_run(&config, &args, &labels);
    }
    let store = open_store(&config, db)?;
    commands::collect::run(&config, &store, &args, &labels).await
}

fn run_diagram(
    config: Option<&std::path::Path>,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    args: DiagramArgs,
) -> Result<()> {
    let document = ConfigDocument::load(config)?;
    let store = offline_store(&document, db, tenant, args.snapshot == "latest", false)?;
    let id = store.resolve_snapshot(&args.snapshot)?;
    let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
    let labels = azdocs::labels::resolve(&config.branding)?;
    commands::diagram::run(&store, &args, &labels)
}

fn run_report(
    config: Option<&std::path::Path>,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    args: ReportArgs,
) -> Result<()> {
    let document = ConfigDocument::load(config)?;
    let store = offline_store(&document, db, tenant, args.snapshot == "latest", false)?;
    let id = store.resolve_snapshot(&args.snapshot)?;
    let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
    commands::report::run(
        &config,
        document.source.as_deref().and_then(std::path::Path::parent),
        &store,
        &args,
    )
}

fn run_browse(
    config: Option<&std::path::Path>,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    snapshot: String,
) -> Result<()> {
    let document = ConfigDocument::load(config)?;
    let store = offline_store(&document, db, tenant, snapshot == "latest", false)?;
    let id = store.resolve_snapshot(&snapshot)?;
    let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
    let labels = azdocs::labels::resolve(&config.branding)?;
    azdocs::tui::run(&store, &snapshot, labels.tui)
}

fn run_snapshots(
    config: Option<&std::path::Path>,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    command: SnapshotsCommand,
) -> Result<()> {
    let document = ConfigDocument::load(config)?;
    let implicit = matches!(
        &command,
        SnapshotsCommand::List { .. } | SnapshotsCommand::Prune { .. }
    ) || matches!(&command, SnapshotsCommand::Show {snapshot, ..} if snapshot == "latest")
        || matches!(&command, SnapshotsCommand::Diff {a,b,..} if a == "latest" || b == "latest");
    // Only the subcommands that change the database open it writably.
    let writable = matches!(
        &command,
        SnapshotsCommand::Delete { .. }
            | SnapshotsCommand::Prune { .. }
            | SnapshotsCommand::Verify { .. }
    );
    let store = offline_store(&document, db, tenant, implicit, writable)?;
    let labels = azdocs::labels::resolve(&document.values.branding)?;
    commands::snapshots::run(&store, &command, &labels)
}

fn run_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(shell, &mut Cli::command(), "azdocs", &mut std::io::stdout());
    Ok(())
}

/// Labels for commands that run before any config exists: the built-in
/// default set, still overridable from the user labels directory.
fn default_labels() -> Result<Labels> {
    Ok(LabelPack::load()?.get(DEFAULT_LABELS)?)
}

/// `--no-color` or a non-empty `NO_COLOR` (https://no-color.org) disables
/// ANSI colour; an empty variable is treated as unset, per that convention.
fn color_enabled(no_color_flag: bool, env: Option<std::ffi::OsString>) -> bool {
    !(no_color_flag || env.is_some_and(|value| !value.is_empty()))
}

fn init_tracing(verbosity: u8, color: bool) {
    let default_level = match verbosity {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("azdocs={default_level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(color)
        .with_writer(std::io::stderr)
        .init();
}

fn open_store(
    config: &Config,
    db_override: Option<&std::path::Path>,
) -> Result<azdocs::store::Store> {
    let path = db_override.unwrap_or(&config.storage.db_path);
    Ok(azdocs::store::Store::open(path)?)
}

fn offline_store(
    document: &ConfigDocument,
    db: Option<&std::path::Path>,
    tenant: Option<&str>,
    implicit: bool,
    writable: bool,
) -> Result<azdocs::store::Store> {
    let mut path = document.values.storage.db_path.clone();
    if path.is_relative()
        && let Some(parent) = document.source.as_deref().and_then(std::path::Path::parent)
    {
        path = parent.join(path);
    }
    let scope = if let Some(reference) = tenant {
        match document.resolve(Some(reference)) {
            Ok(config) => config.auth.tenant_id,
            Err(_) if uuid::Uuid::parse_str(reference).is_ok() => Some(reference.to_owned()),
            Err(error) => return Err(error.into()),
        }
    } else if implicit {
        document.resolve(None)?.auth.tenant_id
    } else {
        None
    };
    let path = db.unwrap_or(&path);
    let store = if writable {
        azdocs::store::Store::open(path)?
    } else {
        azdocs::store::Store::open_read_only(path)?
    };
    Ok(store.with_tenant(scope.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_color_disabled_when_no_color_env_set() {
        assert!(color_enabled(false, None));
        assert!(color_enabled(false, Some(std::ffi::OsString::new())));
        assert!(!color_enabled(false, Some("1".into())));
        assert!(!color_enabled(true, None));
    }
}
