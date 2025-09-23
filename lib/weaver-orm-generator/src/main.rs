// Resolving module naming conflicts
use clap::Parser as _;
use weaver_orm_generator::{generate, load_sql_from_file, load_sql_from_remote};

pub mod generator;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Output directory
  #[arg(short, long, default_value_t = String::from("."))]
  output: String,

  /// Commands
  #[command(subcommand)]
  command: Commands,
}

/// Commands
#[derive(clap::Subcommand, Debug)]
enum Commands {
  /// Generate from local sql script
  Local {
    /// Load from sql script
    #[arg(short, long)]
    load: String,
  },

  /// Generate from remote database
  Remote {
    /// Target database host
    #[arg(short = 'H', long)]
    host: String,

    /// Target database port
    #[arg(short = 'P', long)]
    port: u16,

    /// Username
    #[arg(short = 'u', long)]
    user: String,

    /// Password
    #[arg(short = 'p', long)]
    password: String,

    /// Database
    #[arg(short = 'd', long)]
    database: String,
  },
}

fn main() -> Result<(), std::io::Error> {
  let args = Args::parse();
  let sql = match args.command {
    Commands::Local { load } => load_sql_from_file(&load)?,
    Commands::Remote {
      host,
      port,
      user,
      password,
      database,
    } => load_sql_from_remote(&host, port, &user, &password, &database)?,
  };

  generate(&args.output, sql)
}
