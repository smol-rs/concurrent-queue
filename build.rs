//! Build constants emitted by this script are *not* public API.

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

    if !cfg.probe_rustc_version(1, 56) {
        autocfg::emit("concurrent_queue_no_unwind_in_core");
    }
}
