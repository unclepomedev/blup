use blup::commands;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blup")]
#[command(version)]
#[command(about = "The Blender Version Manager", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a specific version of Blender
    Install {
        /// The version to install (e.g. "5.0.0", "4.2.0")
        #[arg(value_name = "VERSION")]
        target_version: String,

        /// Install from daily builds (experimental)
        #[arg(long)]
        daily: bool,

        /// Skip checksum verification
        #[arg(long)]
        skip_checksum: bool,
    },

    /// List installed Blender versions
    #[command(visible_alias = "ls")]
    List {
        /// List remote versions available for download
        #[arg(long, short = 'r')]
        remote: bool,
    },

    /// Run a specific version of Blender
    Run {
        /// The version to run (optional if default is set)
        #[arg(value_name = "VERSION")]
        target_version: Option<String>,

        /// Path to scripts folder (injects BLENDER_USER_SCRIPTS)
        #[arg(long, value_name = "PATH")]
        scripts: Option<String>,

        /// Arguments to pass to Blender
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "BLENDER_ARGS"
        )]
        args: Vec<String>,
    },

    /// Uninstall a specific version of Blender
    #[command(visible_alias = "rm")]
    Remove {
        /// The version to uninstall
        #[arg(value_name = "VERSION")]
        target_version: String,

        /// Skip confirmation prompt
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Set or show the default Blender version
    Default {
        /// The version to set as default
        #[arg(value_name = "VERSION")]
        target_version: Option<String>,
    },

    /// Show the path to the Blender executable
    Which {
        /// The version to check (optional if context/default is set)
        #[arg(value_name = "VERSION")]
        target_version: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            target_version,
            daily,
            skip_checksum,
        } => {
            commands::install::run(target_version, daily, skip_checksum).await?;
        }
        Commands::List { remote } => {
            commands::list::run(remote).await?;
        }
        Commands::Run {
            target_version,
            scripts,
            args,
        } => {
            commands::run::run(target_version, scripts, args)?;
        }
        Commands::Remove {
            target_version,
            yes,
        } => {
            commands::remove::run(target_version, yes)?;
        }
        Commands::Default { target_version } => {
            commands::default::run(target_version)?;
        }
        Commands::Which { target_version } => {
            commands::which::run(target_version)?;
        }
    }

    Ok(())
}
