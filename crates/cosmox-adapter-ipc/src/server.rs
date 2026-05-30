//! IPC server — Unix domain socket listener, connection handling, graceful shutdown.

use std::{
    error::Error,
    future::Future,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};

use common::Handle;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, tokio::prelude::*};
use tokio::sync::Notify;

use crate::protocol::{
    IpcResponse, REQUEST_TIMEOUT, deserialize_request, serialize_response, write_frame,
};

struct IpcServerState {
    atomic_stop: AtomicBool,
    stop_notify: Notify,
}

static IPC_STATE: LazyLock<IpcServerState> = LazyLock::new(|| IpcServerState {
    atomic_stop: AtomicBool::new(false),
    stop_notify: Notify::new(),
});

pub struct IpcHandle;

impl Handle for IpcHandle {
    async fn stop(&mut self, _graceful: bool) {
        IPC_STATE.atomic_stop.store(true, Ordering::Relaxed);
        IPC_STATE.stop_notify.notify_waiters();
    }
}

async fn handle_connection(stream: interprocess::local_socket::tokio::Stream) {
    let (mut reader, mut writer) = stream.split();

    loop {
        let frame =
            match tokio::time::timeout(REQUEST_TIMEOUT, crate::protocol::read_frame(&mut reader))
                .await
            {
                Ok(Ok(buf)) => buf,
                Ok(Err(e)) => {
                    log::error!("IPC read error: {e}");
                    break;
                }
                Err(_) => {
                    // Timeout — client may have closed; try one more read and exit on failure
                    break;
                }
            };

        let request = match deserialize_request(&frame) {
            Ok(req) => req,
            Err(e) => {
                let resp = IpcResponse::error(e);
                if let Ok(bytes) = serialize_response(&resp) {
                    let _ = write_frame(&mut writer, &bytes).await;
                }
                break;
            }
        };

        let response = crate::handler::dispatch(request).await;
        match serialize_response(&response) {
            Ok(bytes) => {
                if let Err(e) = write_frame(&mut writer, &bytes).await {
                    log::error!("IPC write error: {e}");
                    break;
                }
            }
            Err(e) => {
                let err_resp = IpcResponse::error(e);
                if let Ok(bytes) = serialize_response(&err_resp) {
                    let _ = write_frame(&mut writer, &bytes).await;
                }
            }
        }
    }
}

/// Start the IPC server.
#[allow(clippy::type_complexity)]
/* * REASON FOR BYPASSING CLIPPY:
 * Clippy flags this function with `clippy::type_complexity` and suggests refactoring
 * the return signature into a `type` alias definition. However, due to the upstream
 * compiler limitations tracked in Rust Tracking Issue #63063
 * (https://github.com/rust-lang/rust/issues/63063), stabilizing or utilizing type
 * aliases for this specific `impl Trait` structure is currently blocked or unstable.
 * We locally suppress this warning here because adhering to Clippy's suggestion is
 * explicitly blocked by the compiler's current type solver state.
 */
pub fn server(
    config: &'static cosmox_configuration::Configuration,
) -> Result<
    (
        impl Future<Output = Result<(), Box<dyn Error>>>,
        impl Handle,
    ),
    Box<dyn Error>,
> {
    let handle = IpcHandle;
    let server_fut = start_server(config);
    Ok((server_fut, handle))
}

async fn start_server(_config: &cosmox_configuration::Configuration) -> Result<(), Box<dyn Error>> {
    let printname = "cosmox.sock";

    log::info!("IPC server starting at '{printname}'");

    let name = printname.to_ns_name::<GenericNamespaced>()?;

    let listener = ListenerOptions::new().name(name).create_tokio()?;

    loop {
        if IPC_STATE.atomic_stop.load(Ordering::Relaxed) {
            break;
        }

        tokio::select! {
          biased;
          _ = IPC_STATE.stop_notify.notified() => {
            break;
          }
          res = listener.accept() => {
            let conn = match res {
              Ok(c) => c,
              Err(e) => {
                log::error!("IPC accept error: {e}");
                continue;
              }
            };

            tokio::spawn(async move {
              handle_connection(conn).await;
            });
          }
        }
    }

    IPC_STATE.atomic_stop.store(false, Ordering::Relaxed);
    Ok(())
}
