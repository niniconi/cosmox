use std::{ffi::CStr, os::raw::c_char, sync::OnceLock};

use crate::{
    Api, create_client,
    types::{UserLogin, UserLoginIdent},
};

#[cfg(feature = "direct")]
use crate::transport::direct::DirectApi;
#[cfg(feature = "ipc")]
use crate::transport::ipc::IpcApi;
#[cfg(feature = "web")]
use crate::transport::web::HttpApi;

// Global tokio runtime for FFI blocking

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
}

// Opaque client handle. The backend is chosen once by cosmox_client_new's
// `backend` argument and fixed for the handle's lifetime: the match dispatch
// (instead of `dyn Api`) lets concrete transports expose generic methods
// without a vtable, and only the selected variant owns a live connection.

enum ClientHandle {
    #[cfg(feature = "web")]
    Web(HttpApi),
    #[cfg(feature = "ipc")]
    Ipc(IpcApi),
    #[cfg(feature = "direct")]
    Direct(DirectApi),
}

/// Dispatch a call to the concrete transport stored in `$handle`.
///
/// `$binding` names the transport inside `$call` (e.g. `c => c.login(...)`).
/// Variants are `cfg`-gated, so the match stays exhaustive under any
/// transport feature combination.
macro_rules! dispatch_client {
    ($handle:expr, $binding:ident => $call:expr) => {
        match $handle {
            #[cfg(feature = "web")]
            ClientHandle::Web($binding) => $call,
            #[cfg(feature = "ipc")]
            ClientHandle::Ipc($binding) => $call,
            #[cfg(feature = "direct")]
            ClientHandle::Direct($binding) => $call,
        }
    };
}

fn cstr(ptr: *const c_char) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("")
}

// C ABI exports

/// Create a new cosmox client. Returns an opaque handle (must be freed with cosmox_client_free).
/// Returns null on error.
#[unsafe(no_mangle)]
pub extern "C" fn cosmox_client_new(
    backend: *const c_char,
    hostname: *const c_char,
    port: u16,
) -> *mut std::ffi::c_void {
    let backend_name = cstr(backend);
    let hostname = cstr(hostname);

    match backend_name {
        #[cfg(feature = "web")]
        "web" => Box::into_raw(Box::new(ClientHandle::Web(create_client::<HttpApi>(
            hostname, port,
        )))) as *mut std::ffi::c_void,
        #[cfg(feature = "ipc")]
        "ipc" => Box::into_raw(Box::new(ClientHandle::Ipc(create_client::<IpcApi>(
            hostname, port,
        )))) as *mut std::ffi::c_void,
        #[cfg(feature = "direct")]
        "direct" => Box::into_raw(Box::new(ClientHandle::Direct(create_client::<DirectApi>(
            hostname, port,
        )))) as *mut std::ffi::c_void,
        _ => std::ptr::null_mut(),
    }
}

/// Free a cosmox client created with cosmox_client_new.
#[unsafe(no_mangle)]
pub extern "C" fn cosmox_client_free(ptr: *mut std::ffi::c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr as *mut ClientHandle);
    }
}

/// Login. Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn cosmox_login(
    ptr: *mut std::ffi::c_void,
    username: *const c_char,
    password: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let client = unsafe { &mut *(ptr as *mut ClientHandle) };
    let payload = UserLogin {
        ident: UserLoginIdent::Username(cstr(username).to_string()),
        password: cstr(password).to_string(),
    };
    let result = dispatch_client!(client, c => runtime().block_on(c.login(payload)));
    match result {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
