//! IPC client example for cosmox.
//!
//! Connects to the cosmox IPC server, sends a "GetSystemAbout" request,
//! and prints the response.
//!
//! Usage:
//! ```sh
//! cargo run --example client -- /path/to/cosmox.sock
//! ```

use std::env;

use interprocess::local_socket::{
    GenericNamespaced,
    tokio::{Stream, prelude::*},
};
use rkyv::{from_bytes, rancor, to_bytes};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use cosmox_adapter_ipc::{IpcEndpoint, IpcRequest, IpcResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let socket_path = args.get(1).map(|s| s.as_str()).unwrap_or("cosmox-ipc");

    let name = socket_path.to_ns_name::<GenericNamespaced>()?;
    let mut conn = Stream::connect(name).await?;

    // Build a request: GetSystemAbout with no auth token and no payload
    let request = IpcRequest {
        endpoint: IpcEndpoint::GetSystemAbout,
        token: None,
        payload: Vec::new(),
    };

    // Serialize the request
    let request_bytes =
        to_bytes::<rancor::Error>(&request).map_err(|e| format!("Serialization failed: {e}"))?;
    let request_bytes = request_bytes.into_vec();

    // Write frame: [u64 LE length][rkyv bytes]
    let len = request_bytes.len() as u64;
    conn.write_all(&len.to_le_bytes()).await?;
    conn.write_all(&request_bytes).await?;
    conn.flush().await?;

    // Read response frame: [u64 LE length][rkyv bytes]
    let mut len_buf = [0u8; 8];
    conn.read_exact(&mut len_buf).await?;
    let resp_len = u64::from_le_bytes(len_buf) as usize;

    let mut resp_buf = vec![0u8; resp_len];
    conn.read_exact(&mut resp_buf).await?;

    // Deserialize the response
    let response: IpcResponse = from_bytes::<IpcResponse, rancor::Error>(&resp_buf)
        .map_err(|e| format!("Deserialization failed: {e}"))?;

    if response.success {
        println!("✅ Success");
        if let Some(data) = &response.data {
            // The data is rkyv-serialized bytes of the actual response (String for GetSystemAbout)
            let about: String = from_bytes::<String, rancor::Error>(data)
                .map_err(|e| format!("Response data deserialization failed: {e}"))?;
            println!("About:\n{about}");
        }
    } else {
        println!("❌ Error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
