fn main() {
    let cfg = match autocfg::AutoCfg::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            println!(
                "cargo:warning=concurrent-queue: failed to detect compiler features: {}",
                e
            );
            return;
        }
    };

    // We use "no_*" instead of "has_*" here. For non-Cargo
    // build tools that don't run build.rs, the negative
    // allows us to treat the current Rust version as the
    // latest stable version, for when version information
    // isn't available.
    if !cfg.probe_rustc_version(1, 49) {
        autocfg::emit("cq_no_spin_loop");
    }
}
