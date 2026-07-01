//! In-memory session store keyed by a random opaque token.
//!
//! Tauri commands receive the token via the `session` argument; we look it up,
//! check expiry, and hand back the authenticated user. Persisting sessions to
//! disk is unnecessary for a desktop admin app — restart = re-login.

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub is_super_admin: bool,
    /// Resolved at login to avoid repeated DB hits.
    pub permission_codes: Vec<String>,
    pub role_ids: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<String, AuthenticatedUser>>>,
    ttl_minutes: i64,
}

impl SessionStore {
    pub fn new(ttl_minutes: i64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            ttl_minutes,
        }
    }

    pub fn issue(&self, mut user: AuthenticatedUser) -> AppResult<String> {
        user.expires_at = Utc::now() + Duration::minutes(self.ttl_minutes);
        let token = random_token();
        self.inner.write().insert(token.clone(), user);
        Ok(token)
    }

    pub fn lookup(&self, token: &str) -> AppResult<AuthenticatedUser> {
        let guard = self.inner.read();
        let user = guard
            .get(token)
            .ok_or(AppError::Unauthorized)?
            .clone();
        if user.expires_at < Utc::now() {
            drop(guard);
            self.inner.write().remove(token);
            return Err(AppError::Unauthorized);
        }
        Ok(user)
    }

    pub fn revoke(&self, token: &str) {
        self.inner.write().remove(token);
    }

    /// Used by tests / diagnostics to make sure leaked sessions can't accumulate forever.
    #[allow(dead_code)]
    pub fn gc(&self) {
        let mut guard = self.inner.write();
        let now = Utc::now();
        guard.retain(|_, u| u.expires_at >= now);
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}