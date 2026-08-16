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

fn not_yet_implemented(command: &Command) -> Result<()> {
    let name = match command {
        Command::Collect(_) => "collect",
        Command::Snapshots(_) => "snapshots",
        Command::Report(_) => "report",
        Command::Diagram(_) => "diagram",
        Command::Query(_) => "query list/show",
        Command::Browse { .. } => "browse",
        Command::Init { .. } | Command::Check | Command::Completions { .. } => {
            unreachable!("handled in main")
        }
    };
    anyhow::bail!("`azdocs {name}` is not implemented yet");
}
