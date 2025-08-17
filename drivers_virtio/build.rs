fn main() {
    // Solo para bare-metal
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "none" {
        if std::env::var("CARGO_FEATURE_KERNEL").is_err() {
            panic!("drivers_virtio solo puede compilarse para target_os=none si la feature 'kernel' está activa (como dependencia del kernel)");
        }
    }
}
