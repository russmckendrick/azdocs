use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use azdocs::cli::{Cli, Command, QueryCommand};
use azdocs::commands;
use azdocs::config::Config;
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
        Command::Check => {
            let config = Config::load(cli.config.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::check::run(&config, &labels).await
        }
        Command::Query(QueryCommand::Run {
            query,
            format,
            subscriptions,
        }) => {
            let config = Config::load(cli.config.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::query::run(&config, &query, format, &subscriptions, &labels).await
        }
        Command::Query(QueryCommand::List { category }) => {
            commands::query::list(category.as_deref(), &default_labels()?)
        }
        Command::Query(QueryCommand::Show { name }) => commands::query::show(&name),
        Command::Collect(args) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::collect::run(&config, &store, &args, &labels).await
        }
        Command::Diagram(args) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            commands::diagram::run(&store, &args, &labels)
        }
        Command::Report(args) => {
            let (config, source) = Config::load_with_source(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let config_dir = source.as_deref().and_then(std::path::Path::parent);
            commands::report::run(&config, config_dir, &store, &args)
        }
        Command::Browse { snapshot } => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
            azdocs::tui::run(&store, &snapshot, labels.tui)
        }
        Command::Snapshots(subcommand) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            let labels = azdocs::labels::resolve(&config.branding)?;
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
