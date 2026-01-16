use blup::commands;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blup")]
#[command(about = "The Blender Version Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install { version: String },
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { version } => {
            commands::install::run(version).await?;
        }
        Commands::List => {
            commands::list::run()?;
        }
    }

    Ok(())
}
