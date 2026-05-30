use ffmpeg_sys_next as ff;
use std::{ffi::CStr, ptr};

/// Get the names of all hardware acceleration methods supported at compile
/// time by the FFmpeg build. This does NOT verify driver availability;
/// use `get_real_available_methods()` for runtime validation.
pub fn get_supported_methods() -> Vec<String> {
    let mut methods = Vec::new();
    let mut device_type = ff::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE;

    unsafe {
        loop {
            device_type = ff::av_hwdevice_iterate_types(device_type);

            if device_type == ff::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE {
                break;
            }

            let c_name = ff::av_hwdevice_get_type_name(device_type);
            if !c_name.is_null()
                && let Ok(name_str) = CStr::from_ptr(c_name).to_str()
            {
                methods.push(name_str.to_string());
            }
        }
    }
    methods
}

pub fn get_real_available_methods() -> Vec<String> {
    let mut available_methods = Vec::new();
    let mut device_type = ff::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE;

    unsafe {
        loop {
            // Iterate through compile-time supported types.
            device_type = ff::av_hwdevice_iterate_types(device_type);
            if device_type == ff::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE {
                break;
            }

            // Attempt to create a real hardware device context for this type.
            // This validates that the driver and device are actually available.
            let mut ctx = ptr::null_mut();
            let res =
                ff::av_hwdevice_ctx_create(&mut ctx, device_type, ptr::null(), ptr::null_mut(), 0);

            if res == 0 && !ctx.is_null() {
                let c_name = ff::av_hwdevice_get_type_name(device_type);
                if !c_name.is_null()
                    && let Ok(name_str) = CStr::from_ptr(c_name).to_str()
                {
                    available_methods.push(name_str.to_string());
                }
                // Free temporary context to prevent memory leaks.
                ff::av_buffer_unref(&mut ctx);
            }
        }
    }
    available_methods
}

/// Check if a specific hardware acceleration method (e.g. "cuda" or "vaapi")
/// is available on the current system.
pub fn is_method_supported(method_name: &str) -> bool {
    let available = get_supported_methods();
    available
        .iter()
        .any(|m| m.eq_ignore_ascii_case(method_name))
}
