mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "snowblog", version, about = "Typst blog service")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve(commands::ServeArgs),
    Db(commands::DbArgs),
    Import(commands::ImportArgs),
    Rerender(commands::RerenderArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    match Cli::parse().command {
        Command::Serve(args) => commands::serve(args).await,
        Command::Db(args) => commands::db(args).await,
        Command::Import(args) => commands::import(args).await,
        Command::Rerender(args) => commands::rerender(args).await,
    }
}
