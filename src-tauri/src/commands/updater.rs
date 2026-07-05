//! Auto-update wrapper commands.
//!
//! Tauri 1.6's built-in updater is exposed via `app.updater().check()` which
//! returns `UpdateResponse<R>` (NOT Option). `is_update_available()` reports
//! whether a newer release was found. `download_and_install()` reads the
//! `updater.pubkey` from `tauri.conf.json` internally and verifies the
//! signature against it.
//!
//! `UpdateManifest` mirrors the fields the JS side needs to render the
//! Updater page (current vs latest version, mandatory flag, body). The
//! pubkey lives in tauri.conf.json and is owned by the config layer, not us.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum UpdaterError {
    #[error("no update available")]
    Unavailable,
    #[error("check failed: {0}")]
    Check(String),
    #[error("install failed: {0}")]
    Install(String),
}

impl serde::Serialize for UpdaterError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct UpdateManifest {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
    pub mandatory: bool,
}

pub async fn check(handle: &tauri::AppHandle) -> Result<UpdateManifest, UpdaterError> {
    let updater = handle.updater();
    let current = handle.package_info().version.to_string();
    let response = updater
        .check()
        .await
        .map_err(|e| UpdaterError::Check(e.to_string()))?;
    if response.is_update_available() {
        Ok(UpdateManifest {
            available: true,
            current_version: current,
            latest_version: Some(response.latest_version().to_string()),
            date: response.date().map(|d| d.to_string()),
            body: response.body().map(|b| b.to_string()),
            // Tauri 1.x doesn't surface a `mandatory` flag — treat the absence
            // of a `should_update` flag in latest.json as best-effort. The
            // Rust side still runs the version comparison server-side.
            mandatory: false,
        })
    } else {
        Ok(UpdateManifest {
            available: false,
            current_version: current,
            ..Default::default()
        })
    }
}

pub async fn install(handle: &tauri::AppHandle) -> Result<(), UpdaterError> {
    let updater = handle.updater();
    let response = updater
        .check()
        .await
        .map_err(|e| UpdaterError::Check(e.to_string()))?;
    if !response.is_update_available() {
        return Err(UpdaterError::Unavailable);
    }
    response
        .download_and_install()
        .await
        .map_err(|e| UpdaterError::Install(e.to_string()))
}