#![cfg(feature = "ffmpeg-tests")]

use cosmox_ffmpeg::{hw_accel, init};

#[test]
pub fn get_ava_hw() {
    // Must initialize FFmpeg network and global components first,
    // otherwise the underlying interface might return empty results.
    init();

    println!("Probing hardware acceleration interfaces supported by the current system...");

    let supported = hw_accel::get_supported_methods();

    println!("\n[Probe Results]:");
    if supported.is_empty() {
        println!(
            "No available hardware acceleration interfaces detected; the system will rely entirely on the CPU."
        );
    } else {
        for method in &supported {
            println!("  -> Enabled: {}", method);
        }
    }

    println!("\n[Specific Interface Check]:");
    let target = "cuda";
    if hw_accel::is_method_supported(target) {
        println!(
            "Detected {} available! Can seamlessly enable NVIDIA acceleration.",
            target
        );
    } else {
        println!("{} is not available in the current environment.", target);
    }

    let real_supported = hw_accel::get_real_available_methods();

    println!("\n[Real Available Hardware Results]:");
    for method in real_supported {
        println!("  -> Passed at runtime: {}", method);
    }
}
