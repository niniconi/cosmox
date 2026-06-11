use anyhow::Result;
use clap::Parser;
use cosmox_plugin_packager::{PackFromProfile, pack};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input directory
    #[arg(short, long)]
    input: String,

    /// Output directory
    #[arg(short, long)]
    output: String,

    /// Use release target
    #[arg(long)]
    release: bool,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let target = if args.release {
        PackFromProfile::Release
    } else {
        PackFromProfile::Debug
    };

    log::info!(
        "Packaging plugin from {:?} to {:?}",
        args.input, args.output
    );
    pack(args.input.as_str(), args.output.as_str(), target)?;
    Ok(())
}
