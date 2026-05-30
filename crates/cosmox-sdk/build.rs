fn main() {
    #[cfg(not(any(feature = "native", feature = "ffi")))]
    compile_error!("At least one interface feature must be enabled: \"native\" or \"ffi\"");

    #[cfg(not(any(feature = "web", feature = "ipc", feature = "direct")))]
    compile_error!(
        "At least one transport feature must be enabled: \"web\", \"ipc\", or \"direct\""
    );
}
