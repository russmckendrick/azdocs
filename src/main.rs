use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use azdocs::cli::{Cli, Command, QueryCommand};
use azdocs::commands;
use azdocs::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.no_color);

    match cli.command {
        Command::Init {
            force,
            non_interactive,
        } => commands::init::run(cli.config.as_deref(), force, non_interactive),
        Command::Check => {
            let config = Config::load(cli.config.as_deref())?;
            commands::check::run(&config).await
        }
        Command::Query(QueryCommand::Run {
            query,
            format,
            subscriptions,
        }) => {
            let config = Config::load(cli.config.as_deref())?;
            commands::query::run(&config, &query, format, &subscriptions).await
        }
        Command::Query(QueryCommand::List { category }) => {
            commands::query::list(category.as_deref())
        }
        Command::Query(QueryCommand::Show { name }) => commands::query::show(&name),
        Command::Collect(args) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            commands::collect::run(&config, &store, &args).await
        }
        Command::Diagram(args) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            commands::diagram::run(&store, &args)
        }
        Command::Report(args) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            commands::report::run(&store, &args)
        }
        Command::Snapshots(subcommand) => {
            let config = Config::load(cli.config.as_deref())?;
            let store = open_store(&config, cli.db.as_deref())?;
            commands::snapshots::run(&store, &subcommand)
        }
        Command::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "azdocs", &mut std::io::stdout());
            Ok(())
        }
        command => not_yet_implemented(&command),
    }
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

fn not_yet_implemented(command: &Command) -> Result<()> {
    let name = match command {
        Command::Browse { .. } => "browse",
        _ => unreachable!("handled in main"),
    };
    anyhow::bail!("`azdocs {name}` is not implemented yet");
}
