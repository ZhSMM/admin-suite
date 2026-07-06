//! Tauri app bootstrap. Wires up:
//! - DB connection + Flyway-style migrations on startup.
//! - First-run bootstrap (creates the default super-admin user).
//! - In-memory session store.
//! - All #[tauri::command] handlers.

mod auth;
mod commands;
mod crypto;
mod db;
mod error;
mod llm;
mod models;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::auth::password::hash_password;
use crate::auth::session::SessionStore;
use crate::commands::auth as auth_cmd;
use crate::commands::audit as audit_cmd;
use crate::commands::backup as backup_cmd;
use crate::commands::chat_history as chat_history_cmd;
use crate::commands::crash::{self as crash_cmd, CrashStore};
use crate::commands::llm::{self as llm_cmd};
use crate::commands::llm_fallback;
use crate::commands::menus as menus_cmd;
use crate::commands::metrics::{self as metrics, SharedMetrics};
use crate::commands::migrate_cmd as migrate_cmd;
use crate::commands::permissions as perm_cmd;
use crate::commands::resources as res_cmd;
use crate::commands::roles as roles_cmd;
use crate::commands::settings as settings_cmd;
use crate::commands::updater as updater_cmd;
use crate::commands::users as users_cmd;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::llm::fallback::FallbackManager;

use self::crypto::MasterKey;

/// Shared application state, accessible from every command via `State<AppState>`.
pub struct AppState {
    pub db: Arc<Db>,
    pub sessions: SessionStore,
    pub migrations_dir: PathBuf,
    pub data_dir: PathBuf,
    /// IPC performance registry. Every command starts with a `metrics::time`
    /// guard that records its latency on drop.
    pub metrics: SharedMetrics,
    /// Crash diagnostics store. Persists one JSON per incident to
    /// `<data_dir>/crashes/`. The panic hook is installed in `run()` so every
    /// panic that escapes a command is captured.
    pub crashes: Arc<CrashStore>,
    /// AES-GCM master key for encrypting LLM API keys at rest. Lives only in
    /// memory after being loaded from `<config_dir>/.llm_master_key`.
    pub master_key: Arc<MasterKey>,
    /// Local fallback engine — manages GGUF model download + llama-server
    /// lifecycle. Persists state to `<data_dir>/llm/fallback_state.json`.
    pub fallback: FallbackManager,
}

const DEFAULT_ADMIN_USERNAME: &str = "admin";
const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

// =============================================================
//                       Tauri commands
// =============================================================
//
// Convention: every command takes a `state: State<AppState>` and a `token: String`
// (the session token from login). This makes permission checks uniform: the
// command resolver looks up the session, and individual `commands::*` helpers
// re-check `require_permission` against the resource being touched.

fn map_err<E: Into<AppError>>(e: E) -> AppError {
    e.into()
}

#[tauri::command]
fn auth_login(
    state: State<AppState>,
    username: String,
    password: String,
) -> Result<auth_cmd::LoginResult, AppError> {
    let _t = metrics::time(&state.metrics, "auth_login");
    auth_cmd::login(&state.db, &state.sessions, &username, &password).map_err(map_err)
}

#[tauri::command]
fn auth_logout(state: State<AppState>, token: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "auth_logout");
    auth_cmd::logout(&state.sessions, &token);
    Ok(())
}

#[tauri::command]
fn auth_me(state: State<AppState>, token: String) -> Result<models::UserSafe, AppError> {
    let _t = metrics::time(&state.metrics, "auth_me");
    auth_cmd::current_user(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn users_list(
    state: State<AppState>,
    token: String,
    query: Option<models::UserListQuery>,
) -> Result<models::UserListResult, AppError> {
    let _t = metrics::time(&state.metrics, "users_list");
    users_cmd::list(&state.db, &state.sessions, &token, query.unwrap_or(models::UserListQuery {
        keyword: None, status: None, role_id: None, page: None, page_size: None,
    }))
    .map_err(map_err)
}

#[tauri::command]
fn users_get(state: State<AppState>, token: String, id: String) -> Result<models::UserSafe, AppError> {
    let _t = metrics::time(&state.metrics, "users_get");
    users_cmd::get(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[tauri::command]
fn users_create(
    state: State<AppState>,
    token: String,
    payload: models::UserCreate,
) -> Result<models::UserSafe, AppError> {
    let _t = metrics::time(&state.metrics, "users_create");
    users_cmd::create(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn users_update(
    state: State<AppState>,
    token: String,
    payload: models::UserUpdate,
) -> Result<models::UserSafe, AppError> {
    let _t = metrics::time(&state.metrics, "users_update");
    users_cmd::update(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn users_delete(state: State<AppState>, token: String, id: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "users_delete");
    users_cmd::delete(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[tauri::command]
fn roles_list(state: State<AppState>, token: String) -> Result<Vec<models::Role>, AppError> {
    let _t = metrics::time(&state.metrics, "roles_list");
    roles_cmd::list(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn roles_get(state: State<AppState>, token: String, id: String) -> Result<models::Role, AppError> {
    let _t = metrics::time(&state.metrics, "roles_get");
    roles_cmd::get(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[tauri::command]
fn roles_create(
    state: State<AppState>,
    token: String,
    payload: models::RoleCreate,
) -> Result<models::Role, AppError> {
    let _t = metrics::time(&state.metrics, "roles_create");
    roles_cmd::create(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn roles_update(
    state: State<AppState>,
    token: String,
    payload: models::RoleUpdate,
) -> Result<models::Role, AppError> {
    let _t = metrics::time(&state.metrics, "roles_update");
    roles_cmd::update(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn roles_delete(state: State<AppState>, token: String, id: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "roles_delete");
    roles_cmd::delete(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[tauri::command]
fn roles_assign_menus(
    state: State<AppState>,
    token: String,
    role_id: String,
    menu_ids: Vec<String>,
) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "roles_assign_menus");
    roles_cmd::assign_menus(&state.db, &state.sessions, &token, &role_id, menu_ids).map_err(map_err)
}

#[tauri::command]
fn roles_get_menus(
    state: State<AppState>,
    token: String,
    role_id: String,
) -> Result<Vec<String>, AppError> {
    let _t = metrics::time(&state.metrics, "roles_get_menus");
    roles_cmd::get_role_menus(&state.db, &state.sessions, &token, &role_id).map_err(map_err)
}

#[tauri::command]
fn permissions_list(state: State<AppState>, token: String) -> Result<Vec<models::Permission>, AppError> {
    let _t = metrics::time(&state.metrics, "permissions_list");
    perm_cmd::list(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn menus_tree(state: State<AppState>, token: String) -> Result<Vec<models::MenuNode>, AppError> {
    let _t = metrics::time(&state.metrics, "menus_tree");
    menus_cmd::tree(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn menus_create(
    state: State<AppState>,
    token: String,
    payload: models::MenuCreate,
) -> Result<models::Menu, AppError> {
    let _t = metrics::time(&state.metrics, "menus_create");
    menus_cmd::create(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn menus_update(
    state: State<AppState>,
    token: String,
    payload: models::MenuUpdate,
) -> Result<models::Menu, AppError> {
    let _t = metrics::time(&state.metrics, "menus_update");
    menus_cmd::update(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn menus_delete(state: State<AppState>, token: String, id: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "menus_delete");
    menus_cmd::delete(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceListResponse {
    pub items: Vec<models::Resource>,
    pub active: Option<models::Resource>,
}

#[tauri::command]
fn resources_list(
    state: State<AppState>,
    token: String,
    resource_type: String,
) -> Result<ResourceListResponse, AppError> {
    let _t = metrics::time(&state.metrics, "resources_list");
    let items = res_cmd::list(&state.db, &state.sessions, &token, &resource_type)?;
    let active = res_cmd::get_active(&state.db, &resource_type)?;
    Ok(ResourceListResponse { items, active })
}

#[tauri::command]
fn resources_import_theme(
    state: State<AppState>,
    token: String,
    raw_json: String,
) -> Result<models::Resource, AppError> {
    let _t = metrics::time(&state.metrics, "resources_import_theme");
    let (code, name, content) = res_cmd::parse_import("theme", &raw_json)?;
    res_cmd::import_theme(
        &state.db,
        &state.sessions,
        &token,
        models::ResourceImport { resource_type: "theme".into(), code, name, content },
    )
    .map_err(map_err)
}

#[tauri::command]
fn resources_import_locale(
    state: State<AppState>,
    token: String,
    raw_json: String,
) -> Result<models::Resource, AppError> {
    let _t = metrics::time(&state.metrics, "resources_import_locale");
    let (code, name, content) = res_cmd::parse_import("locale", &raw_json)?;
    res_cmd::import_locale(
        &state.db,
        &state.sessions,
        &token,
        models::ResourceImport { resource_type: "locale".into(), code, name, content },
    )
    .map_err(map_err)
}

#[tauri::command]
fn resources_activate(
    state: State<AppState>,
    token: String,
    resource_type: String,
    code: String,
) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "resources_activate");
    res_cmd::activate(&state.db, &state.sessions, &token, &resource_type, &code).map_err(map_err)
}

#[tauri::command]
fn resources_delete(state: State<AppState>, token: String, id: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "resources_delete");
    res_cmd::delete(&state.db, &state.sessions, &token, &id).map_err(map_err)
}

#[tauri::command]
fn resources_update(
    state: State<AppState>,
    token: String,
    payload: models::ResourceUpdate,
) -> Result<models::Resource, AppError> {
    let _t = metrics::time(&state.metrics, "resources_update");
    res_cmd::update(&state.db, &state.sessions, &token, payload).map_err(map_err)
}

#[tauri::command]
fn audit_list(
    state: State<AppState>,
    token: String,
    query: Option<models::AuditQuery>,
) -> Result<audit_cmd::AuditListResult, AppError> {
    let _t = metrics::time(&state.metrics, "audit_list");
    audit_cmd::list(
        &state.db,
        &state.sessions,
        &token,
        query.unwrap_or(models::AuditQuery {
            action: None,
            actor_id: None,
            resource: None,
            payload_search: None,
            from: None,
            to: None,
            page: None,
            page_size: None,
        }),
    )
    .map_err(map_err)
}

#[tauri::command]
fn migrate_run(state: State<AppState>) -> Result<migrate_cmd::MigrateResult, AppError> {
    let _t = metrics::time(&state.metrics, "migrate_run");
    migrate_cmd::run(&state.db, state.migrations_dir.clone()).map_err(map_err)
}

#[tauri::command]
fn migrate_status(state: State<AppState>) -> Result<Vec<crate::db::migrate::MigrationStatus>, AppError> {
    let _t = metrics::time(&state.metrics, "migrate_status");
    migrate_cmd::status(&state.db, state.migrations_dir.clone()).map_err(map_err)
}

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub data_dir: String,
    pub db_path: String,
    pub migrations_dir: String,
    pub default_admin: AdminInfo,
}

#[derive(Debug, Serialize)]
pub struct AdminInfo {
    pub username: String,
    pub password: String,
    pub note: &'static str,
}

#[tauri::command]
fn app_info(state: State<AppState>) -> Result<AppInfo, AppError> {
    let _t = metrics::time(&state.metrics, "app_info");
    Ok(AppInfo {
        data_dir: state.data_dir.to_string_lossy().to_string(),
        db_path: state.db.path().to_string_lossy().to_string(),
        migrations_dir: state.migrations_dir.to_string_lossy().to_string(),
        default_admin: AdminInfo {
            username: DEFAULT_ADMIN_USERNAME.into(),
            password: DEFAULT_ADMIN_PASSWORD.into(),
            note: "Change this password on first login.",
        },
    })
}

// =============================================================
// Settings
// =============================================================

#[tauri::command]
fn settings_list(
    state: State<AppState>,
    token: String,
) -> Result<Vec<settings_cmd::Setting>, AppError> {
    let _t = metrics::time(&state.metrics, "settings_list");
    settings_cmd::list(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn settings_set(
    state: State<AppState>,
    token: String,
    updates: Vec<settings_cmd::SettingUpdate>,
) -> Result<Vec<settings_cmd::Setting>, AppError> {
    let _t = metrics::time(&state.metrics, "settings_set");
    settings_cmd::set_many(&state.db, &state.sessions, &token, updates).map_err(map_err)
}

#[tauri::command]
fn settings_list_public(
    state: State<AppState>,
    token: String,
) -> Result<Vec<settings_cmd::Setting>, AppError> {
    let _t = metrics::time(&state.metrics, "settings_list_public");
    settings_cmd::list_public(&state.db, &state.sessions, &token).map_err(map_err)
}

// =============================================================
// Backups
// =============================================================

#[tauri::command]
fn backup_list(state: State<AppState>, token: String) -> Result<Vec<backup_cmd::BackupInfo>, AppError> {
    let _t = metrics::time(&state.metrics, "backup_list");
    backup_cmd::list(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn backup_create(state: State<AppState>, token: String) -> Result<backup_cmd::BackupInfo, AppError> {
    let _t = metrics::time(&state.metrics, "backup_create");
    backup_cmd::create(&state.db, &state.sessions, &token).map_err(map_err)
}

#[tauri::command]
fn backup_delete(state: State<AppState>, token: String, name: String) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "backup_delete");
    backup_cmd::delete(&state.db, &state.sessions, &token, &name).map_err(map_err)
}

#[tauri::command]
fn backup_restore(
    state: State<AppState>,
    token: String,
    name: String,
) -> Result<backup_cmd::RestoreRequest, AppError> {
    let _t = metrics::time(&state.metrics, "backup_restore");
    backup_cmd::restore(&state.db, &state.sessions, &token, &name).map_err(map_err)
}

// =============================================================
// IPC performance monitoring
// =============================================================
//
// `metrics_snapshot` / `metrics_clear` are wrapped too — but the snapshot
// itself is what feeds them, so the UI may want to filter these out to avoid
// recursion noise.  We don't bother — the round-trip is microseconds.

#[tauri::command]
fn metrics_snapshot(state: State<AppState>) -> Vec<metrics::IpcMetric> {
    let _t = metrics::time(&state.metrics, "metrics_snapshot");
    state.metrics.snapshot()
}

#[tauri::command]
fn metrics_clear(state: State<AppState>) {
    let _t = metrics::time(&state.metrics, "metrics_clear");
    state.metrics.clear();
}

// =============================================================
// Crash diagnostics
// =============================================================
//
// `crash_log` accepts frontend-reported incidents (Vue errorHandler +
// window.onunhandledrejection). The Rust panic hook bypasses this and writes
// directly to the store. List / get / clear require `diagnostics:read` /
// `diagnostics:clear`.

#[tauri::command]
fn crash_log(state: State<AppState>, input: crash_cmd::CrashReportInput) -> Result<crash_cmd::CrashReport, AppError> {
    let _t = metrics::time(&state.metrics, "crash_log");
    state.crashes.record(input).map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
fn crash_list(state: State<AppState>, token: String) -> Result<Vec<crash_cmd::CrashReport>, AppError> {
    let _t = metrics::time(&state.metrics, "crash_list");
    let user = state.sessions.lookup(&token).map_err(map_err)?;
    crate::auth::rbac::require_permission(&user, "diagnostics:read").map_err(map_err)?;
    state.crashes.list().map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
fn crash_clear(state: State<AppState>, token: String) -> Result<usize, AppError> {
    let _t = metrics::time(&state.metrics, "crash_clear");
    let user = state.sessions.lookup(&token).map_err(map_err)?;
    crate::auth::rbac::require_permission(&user, "diagnostics:clear").map_err(map_err)?;
    state.crashes.clear().map_err(|e| AppError::Internal(e.to_string()))
}

// =============================================================
// Auto-update
// =============================================================
//
// Two commands: `updater_check` (returns the latest manifest) and
// `updater_install` (downloads + stages the new binary; the user must
// restart to actually swap).  Both require `updater:check` / `updater:apply`.

#[tauri::command]
async fn updater_check(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    token: String,
) -> Result<updater_cmd::UpdateManifest, AppError> {
    let _t = metrics::time(&state.metrics, "updater_check");
    let user = state.sessions.lookup(&token).map_err(map_err)?;
    crate::auth::rbac::require_permission(&user, "updater:check").map_err(map_err)?;
    updater_cmd::check(&app)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
async fn updater_install(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    token: String,
) -> Result<(), AppError> {
    let _t = metrics::time(&state.metrics, "updater_install");
    let user = state.sessions.lookup(&token).map_err(map_err)?;
    crate::auth::rbac::require_permission(&user, "updater:apply").map_err(map_err)?;
    updater_cmd::install(&app)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

// =============================================================
//                         Bootstrap
// =============================================================

fn data_dir() -> PathBuf {
    // Tauri exposes `path_resolver` from the AppHandle; for this scaffold we
    // honour an env override so tests can point at a temp dir.
    if let Ok(p) = std::env::var("ADMIN_SUITE_DATA_DIR") {
        return PathBuf::from(p);
    }
    // Reasonable default per-platform: $HOME/.admin-suite
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".admin-suite")
}

fn bootstrap(db: &Db, _sessions: &SessionStore) -> AppResult<()> {
    // Run migrations (idempotent).
    let dir = db::migrate::resolve_migrations_dir(None)?;
    let applied = db::migrate::run_migrations(db, &dir)?;
    if !applied.is_empty() {
        log::info!("applied {} migration(s)", applied.len());
        for m in &applied {
            log::info!(
                "  + {} ({}, {} ms)",
                m.script,
                m.version.as_deref().unwrap_or("R"),
                m.execution_time_ms
            );
        }
    }
    ensure_default_admin(db)?;
    Ok(())
}

fn ensure_default_admin(db: &Db) -> AppResult<()> {
    let exists: i64 = db.with_conn(|c| {
        let n: i64 =
            c.query_row("SELECT COUNT(*) FROM users WHERE username = ?1", [DEFAULT_ADMIN_USERNAME], |r| {
                r.get(0)
            })?;
        Ok(n)
    })?;
    if exists > 0 {
        return Ok(());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%fZ")
        .to_string();
    let hash = hash_password(DEFAULT_ADMIN_PASSWORD)?;

    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO users (id, username, display_name, password_hash, status,
                                is_super_admin, created_at, updated_at)
             VALUES (?, ?, 'Administrator', ?, 'active', 1, ?, ?)",
            rusqlite::params![id, DEFAULT_ADMIN_USERNAME, hash, now, now],
        )?;
        tx.execute(
            "INSERT INTO user_roles (user_id, role_id) VALUES (?, 'r_super_admin')",
            rusqlite::params![id],
        )?;
        // Auto-grant every management menu to super admin via role_menus.
        let mut stmt = tx.prepare("SELECT id FROM menus")?;
        let menu_ids: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        for menu_id in menu_ids {
            tx.execute(
                "INSERT OR IGNORE INTO role_menus (role_id, menu_id) VALUES ('r_super_admin', ?)",
                rusqlite::params![menu_id],
            )?;
        }
        Ok(())
    })?;
    log::info!(
        "bootstrapped default super-admin: {} / {}",
        DEFAULT_ADMIN_USERNAME,
        DEFAULT_ADMIN_PASSWORD
    );
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = data_dir.join("admin-suite.sqlite");

    // Apply a pending restore BEFORE we open the live DB — once the connection
    // is up the file is locked on Windows and the rename would fail.
    match backup_cmd::apply_pending_restore(&db_path) {
        Ok(Some(prev)) => log::warn!("restored DB from backup; previous copy kept at {}", prev),
        Ok(None) => {}
        Err(e) => log::error!("pending restore failed: {}", e),
    }

    let db = Db::open(&db_path).expect("open db");

    // Session TTL comes from settings; if absent, fall back to 8h so a fresh
    // install with a not-yet-migrated DB doesn't panic.
    let session_minutes: i64 = settings_cmd::get_or(&db, "session.timeout_minutes", "480")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(480);
    let sessions = SessionStore::new(session_minutes);

    if let Err(e) = bootstrap(&db, &sessions) {
        eprintln!("bootstrap failed: {}", e);
        std::process::exit(1);
    }

    // Auto-backup is best-effort: a failure here should not block the app from
    // starting (e.g. read-only data dir).  We log and continue.
    match backup_cmd::maybe_auto_backup(&db, &data_dir) {
        Ok(Some(info)) => log::info!("auto-backup: {}", info.name),
        Ok(None) => {}
        Err(e) => log::warn!("auto-backup skipped: {}", e),
    }

    let migrations_dir = db::migrate::resolve_migrations_dir(None).unwrap_or_else(|_| {
        data_dir.join("migrations")
    });

    let config_dir = data_dir.join("config");
    let master_key = MasterKey::load_or_create(&config_dir).expect("init master key");
    let fallback = FallbackManager::new(data_dir.clone());

    let state = AppState {
        db,
        sessions,
        migrations_dir,
        data_dir: data_dir.clone(),
        metrics: metrics::MetricsRegistry::shared(),
        crashes: Arc::new(
            crash_cmd::CrashStore::new(&data_dir)
                .expect("init crash store"),
        ),
        master_key: Arc::new(master_key),
        fallback,
    };

    // Install the panic hook AFTER CrashStore is constructed — any panic that
    // escapes a command from here on will land in <data_dir>/crashes/.
    crash_cmd::install_panic_hook(state.crashes.clone());

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            // Make sure the main window is shown even if the dev server is slow.
            if let Some(win) = app.get_window("main") {
                let _ = win.show();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth_login,
            auth_logout,
            auth_me,
            users_list,
            users_get,
            users_create,
            users_update,
            users_delete,
            roles_list,
            roles_get,
            roles_create,
            roles_update,
            roles_delete,
            roles_assign_menus,
            roles_get_menus,
            permissions_list,
            menus_tree,
            menus_create,
            menus_update,
            menus_delete,
            resources_list,
            resources_import_theme,
            resources_import_locale,
            resources_activate,
            resources_delete,
            resources_update,
            audit_list,
            migrate_run,
            migrate_status,
            app_info,
            settings_list,
            settings_list_public,
            settings_set,
            backup_list,
            backup_create,
            backup_delete,
            backup_restore,
            metrics_snapshot,
            metrics_clear,
            crash_log,
            crash_list,
            crash_clear,
            updater_check,
            updater_install,
            // LLM v0.6.0
            llm_cmd::llm_providers_list,
            llm_cmd::llm_providers_get,
            llm_cmd::llm_providers_create,
            llm_cmd::llm_providers_update,
            llm_cmd::llm_providers_delete,
            llm_cmd::llm_models_list,
            llm_cmd::llm_models_get,
            llm_cmd::llm_models_create,
            llm_cmd::llm_models_update,
            llm_cmd::llm_models_delete,
            llm_cmd::llm_chat,
            llm_cmd::llm_chat_stream,
            llm_cmd::llm_usage_query,
            llm_cmd::llm_fallback_status,
            llm_cmd::llm_fallback_select_model,
            llm_cmd::llm_fallback_set_enabled,
            llm_cmd::llm_fallback_dismiss_startup_prompt,
            llm_cmd::llm_fallback_startup_prompt_needed,
            // LLM v0.6.2 — one-click local install
            llm_fallback::llm_fallback_install_start,
            llm_fallback::llm_fallback_install_cancel,
            llm_fallback::llm_fallback_server_start,
            llm_fallback::llm_fallback_server_stop,
            llm_fallback::llm_fallback_remove,
            llm_fallback::llm_fallback_disk_free,
            llm_fallback::llm_fallback_discover_trending,
            llm_fallback::llm_fallback_speed_test,
            llm_fallback::llm_fallback_import_local,
            // v0.7.0 — persistent multi-level chat history
            chat_history_cmd::chat_session_list,
            chat_history_cmd::chat_session_create,
            chat_history_cmd::chat_session_update,
            chat_history_cmd::chat_session_delete,
            chat_history_cmd::chat_session_get,
            chat_history_cmd::chat_message_append,
            chat_history_cmd::chat_message_update,
            chat_history_cmd::chat_message_delete,
            chat_history_cmd::chat_session_export,
            // v0.7.1 — provider model discovery
            llm_cmd::llm_provider_list_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}