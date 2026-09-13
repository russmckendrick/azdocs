use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use azdocs::cli::{Cli, Command, QueryCommand};
use azdocs::commands;
use azdocs::config::{Config, ConfigDocument};
use azdocs::labels::{DEFAULT_LABELS, LabelPack, Labels};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.no_color);

    match cli.command {
        Command::Init {
            force,
            non_interactive,
        } => commands::init::run(
            cli.config.as_deref(),
            force,
            non_interactive,
            &default_labels()?,
        ),
        Command::Config(command) => commands::config::run(
            &command,
            cli.config.as_deref(),
            cli.tenant.as_deref(),
            &default_labels()?,
        ),
        Command::Check => {
            let config =
                ConfigDocument::load(cli.config.as_deref())?.resolve(cli.tenant.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::check::run(&config, &labels).await
        }
        Command::Query(QueryCommand::Run {
            query,
            format,
            subscriptions,
        }) => {
            let config =
                ConfigDocument::load(cli.config.as_deref())?.resolve(cli.tenant.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::query::run(&config, &query, format, &subscriptions, &labels).await
        }
        Command::Query(QueryCommand::List { category }) => {
            commands::query::list(category.as_deref(), &default_labels()?)
        }
        Command::Query(QueryCommand::Show { name }) => commands::query::show(&name),
        Command::Collect(args) => {
            let config =
                ConfigDocument::load(cli.config.as_deref())?.resolve(cli.tenant.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::collect::run(&config, &store, &args, &labels).await
        }
        Command::Diagram(args) => {
            let document = ConfigDocument::load(cli.config.as_deref())?;
            let store = offline_store(
                &document,
                cli.db.as_deref(),
                cli.tenant.as_deref(),
                args.snapshot == "latest",
            )?;
            let id = store.resolve_snapshot(&args.snapshot)?;
            let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::diagram::run(&store, &args, &labels)
        }
        Command::Report(args) => {
            let document = ConfigDocument::load(cli.config.as_deref())?;
            let store = offline_store(
                &document,
                cli.db.as_deref(),
                cli.tenant.as_deref(),
                args.snapshot == "latest",
            )?;
            let id = store.resolve_snapshot(&args.snapshot)?;
            let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
            commands::report::run(
                &config,
                document.source.as_deref().and_then(std::path::Path::parent),
                &store,
                &args,
            )
        }
        Command::Browse { snapshot } => {
            let document = ConfigDocument::load(cli.config.as_deref())?;
            let store = offline_store(
                &document,
                cli.db.as_deref(),
                cli.tenant.as_deref(),
                snapshot == "latest",
            )?;
            let id = store.resolve_snapshot(&snapshot)?;
            let config = document.for_snapshot(&store.get_snapshot(&id)?.tenant_id)?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            azdocs::tui::run(&store, &snapshot, labels.tui)
        }
        Command::Snapshots(subcommand) => {
            let document = ConfigDocument::load(cli.config.as_deref())?;
            let implicit = matches!(
                &subcommand,
                azdocs::cli::SnapshotsCommand::List | azdocs::cli::SnapshotsCommand::Prune { .. }
            ) || matches!(&subcommand, azdocs::cli::SnapshotsCommand::Show {snapshot} if snapshot == "latest")
                || matches!(&subcommand, azdocs::cli::SnapshotsCommand::Diff {a,b,..} if a == "latest" || b == "latest");
            let store = offline_store(
                &document,
                cli.db.as_deref(),
                cli.tenant.as_deref(),
                implicit,
            )?;
            let labels = azdocs::labels::resolve(&document.values.branding)?;
            commands::snapshots::run(&store, &subcommand, &labels)
        }
        Command::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "azdocs", &mut std::io::stdout());
            Ok(())
        }
    }
}

/// Labels for commands that run before any config exists: the built-in
/// default set, still overridable from the user labels directory.
fn default_labels() -> Result<Labels> {
    Ok(LabelPack::load()?.get(DEFAULT_LABELS)?)
}

fn init_tracing(verbosity: u8, no_color: bool) {
    let default_level = match verbosity {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("azdocs={default_level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(!no_color)
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
    Ok(azdocs::store::Store::open(db.unwrap_or(&path))?.with_tenant(scope.as_deref()))
}
