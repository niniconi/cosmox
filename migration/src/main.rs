#[cfg(feature = "runtime-async-std")]
use sea_orm_migration::prelude::*;

#[cfg(feature = "runtime-async-std")]
#[async_std::main]
async fn main() {
  cli::run_cli(migration::Migrator).await;
}
