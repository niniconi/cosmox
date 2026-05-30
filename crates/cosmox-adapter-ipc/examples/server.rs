//! IPC server example for cosmox.
//!
//! Starts the IPC server using the global configuration.
//! Run this alongside the client example to test the IPC protocol.
//!
//! Usage:
//! ```sh
//! cargo run --example server
//! ```

use cosmox_configuration::Configuration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Configuration::get_global_configuration();
    let (_server, handle) = cosmox_adapter_ipc::server(config)?;
    // In a real setup, _server.await would drive the server loop.
    // Here we just keep the handle alive.
    drop(handle);
    Ok(())
}
