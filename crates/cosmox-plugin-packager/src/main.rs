use anyhow::Result;
use clap::{Parser, Subcommand};
use cosmox_plugin_packager::{PackFromProfile, pack, unpack};
use log::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Package a plugin source directory into a .tar.gz archive
    Pack {
        /// Input plugin source directory
        #[arg(short, long)]
        input: String,

        /// Output directory for the archive
        #[arg(short, long)]
        output: String,

        /// Use release build profile
        #[arg(long)]
        release: bool,
    },
    /// Unpack a .tar.gz plugin archive into a directory
    Unpack {
        /// Path to the .tar.gz archive
        #[arg(short, long)]
        input: String,

        /// Output directory for extraction
        #[arg(short, long)]
        output: String,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Command::Pack {
            input,
            output,
            release,
        } => {
            let target = if release {
                PackFromProfile::Release
            } else {
                PackFromProfile::Debug
            };
            info!("Packaging plugin from {:?} to {:?}", input, output);
            pack(input.as_str(), output.as_str(), target)?;
        }
        Command::Unpack { input, output } => {
            info!("Unpacking archive {:?} to {:?}", input, output);
            unpack(input.as_str(), output.as_str())?;
        }
    }

    Ok(())
}
