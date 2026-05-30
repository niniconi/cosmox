use std::{env, error::Error};

use common::Handle;
use cosmox_configuration::Configuration;
use cosmox_plugin_manager::plugin_manager::PluginManager;
use tokio::signal::unix::{SignalKind, signal};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn spawn_all_init() -> Result<(), Box<dyn Error>> {
    PluginManager::init().await;
    cosmox_backend_data::init().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Configuration::get_global_configuration();

    println!(include_str!("../banner.txt"), env!("CARGO_PKG_VERSION"));

    let file_appender =
        RollingFileAppender::new(Rotation::DAILY, config.cosmox.log.path.as_str(), "app.log");
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);
    // _guard held for main lifetime — keeps non-blocking writer alive

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(
                #[cfg(debug_assertions)]
                |_| "trace".into(),
                #[cfg(not(debug_assertions))]
                |_| "info".into(),
            ),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_appender))
        .init();

    let server_host = config.server.host.as_ref();
    let server_port = config.server.port;

    log::info!("start");

    log::debug!(
        "Config dump:\n{}",
        serde_json::to_string_pretty(config).expect("serialize config")
    );

    spawn_all_init().await?;

    let (web_server, mut web_handle) =
        cosmox_adapter_web::server(config, server_host, server_port)?;
    let (ipc_server, mut ipc_handle) = cosmox_adapter_ipc::server(config)?;

    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).expect("create SIGINT handler"); // Ctrl+C
        let mut sigterm = signal(SignalKind::terminate()).expect("create SIGTERM handler"); // kill cmd
        let mut sigquit = signal(SignalKind::quit()).expect("create SIGQUIT handler"); // Ctrl+\

        tokio::select! {
          _ = sigint.recv() => {},
          _ = sigterm.recv() => {},
          _ = sigquit.recv() => {},
        }

        web_handle.stop(true).await;
        ipc_handle.stop(true).await;
    });

    let (web, ipc) = tokio::join!(web_server, ipc_server);
    web?;
    ipc?;
    Ok(())
}
