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
    Install {
        version: String,
    },
    List,
    Run {
        version: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Remove {
        version: String,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    Default {
        version: Option<String>,
    },
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
        Commands::Run { version, args } => {
            commands::run::run(version, args)?;
        }
        Commands::Remove { version, yes } => {
            commands::remove::run(version, yes)?;
        }
        Commands::Default { version } => {
            commands::default::run(version)?;
        }
    }

    Ok(())
}
