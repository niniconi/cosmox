use std::{ffi::CStr, os::raw::c_char, sync::OnceLock};

use crate::{
    Api, create_client,
    types::{UserLogin, UserLoginIdent},
};

#[cfg(feature = "web")]
use crate::transport::web::HttpApi;
#[cfg(feature = "ipc")]
use crate::transport::ipc::IpcApi;
#[cfg(feature = "direct")]
use crate::transport::direct::DirectApi;

// Global tokio runtime for FFI blocking

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
}

// Opaque client handle

type ClientHandle = Box<dyn Api>;

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
        "web" => Box::into_raw(Box::new(create_client::<HttpApi>(hostname, port)))
            as *mut std::ffi::c_void,
        "ipc" => Box::into_raw(Box::new(create_client::<IpcApi>(hostname, port)))
            as *mut std::ffi::c_void,
        "direct" => Box::into_raw(Box::new(create_client::<DirectApi>(
            hostname, port,
        ))) as *mut std::ffi::c_void,
        _ => return std::ptr::null_mut(),
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
    match runtime().block_on(client.login(payload)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
