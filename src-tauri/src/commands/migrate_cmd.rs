//! Tauri command wrappers around the Flyway-style migrator.
//! Exposed to the admin UI so admins can see applied/pending migrations.

use crate::db::{self, Db};
use crate::error::AppResult;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct MigrateResult {
    pub applied: Vec<crate::db::migrate::MigrationRecord>,
    pub status: Vec<crate::db::migrate::MigrationStatus>,
}

pub fn run(db: &Db, migrations_dir: PathBuf) -> AppResult<MigrateResult> {
    let dir = db::migrate::resolve_migrations_dir(Some(migrations_dir))?;
    let applied = db::migrate::run_migrations(db, &dir)?;
    let status = db::migrate::status(db, &dir)?;
    Ok(MigrateResult { applied, status })
}

pub fn status(db: &Db, migrations_dir: PathBuf) -> AppResult<Vec<crate::db::migrate::MigrationStatus>> {
    let dir = db::migrate::resolve_migrations_dir(Some(migrations_dir))?;
    db::migrate::status(db, &dir)
}