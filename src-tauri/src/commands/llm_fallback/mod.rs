//! `commands::llm_fallback::*` — One-click offline GGUF installer.
//!
//! Frontend calls into these commands to drive the full lifecycle:
//!
//! 1. `llm_fallback_install_start`  — kick off the two-stage download
//!    (server binary, then GGUF model). Emits `llm:fallback:progress`
//!    events for the UI to render a single weighted progress bar.
//! 2. `llm_fallback_install_cancel` — flip the cancel flag.
//! 3. `llm_fallback_server_start`   — spawn llama-server, upsert the
//!    `llm_providers` row of `kind = "fallback"` so the rest of the
//!    system sees it.
//! 4. `llm_fallback_server_stop`    — kill llama-server, mark provider
//!    row `enabled = 0`.
//! 5. `llm_fallback_remove`         — delete model + reset state.
//!
//! ## Module layout
//!
//! - [`events`] — progress / done payloads + `emit_*` helpers.
//! - [`install`] — `install_start` / `install_cancel` / `run_install`.
//! - [`server`] — `server_start` / `server_stop` / `remove` /
//!   `import_local` / `disk_free` / `discover_trending`.
//! - [`provider`] — DB helpers (`probe_free_port`, `upsert…`, …).
//! - [`speed_test`] — `speed_test` + streaming event payloads.
//! - [`reroute`] — `maybe_reroute_to_local` (called from chat commands).
//!
//! Status snapshot for the UI is the existing `llm_fallback_status`
//! command (no change). Frontend re-fetches it whenever
//! `llm:fallback:progress` fires.

pub mod events;
pub mod install;
pub mod provider;
pub mod reroute;
pub mod server;
pub mod speed_test;

// Re-export every command under the module's path so the parent
// `commands::mod.rs` can register them with `use commands::llm_fallback`
// imports.
pub use install::{
    llm_fallback_install_cancel, llm_fallback_install_start, InstallStartResult,
};
pub use reroute::maybe_reroute_to_local;
pub use server::{
    llm_fallback_disk_free, llm_fallback_discover_trending, llm_fallback_import_local,
    llm_fallback_remove, llm_fallback_server_start, llm_fallback_server_stop,
};
pub use speed_test::{llm_fallback_speed_test, llm_fallback_speed_test_cancel};
