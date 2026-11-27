#![allow(unused_imports)]
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

use crate::models::{AuditEntry, BanInfo, Community, ConfigEntry, Conversation, ConversationRank, Document, IpBan, LinkedDevice, LinkCode, Message, PushSubscription, Role, SessionInfo, SuspensionInfo, Title, User, UserIpHistory, Web};

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}


impl Db {
    pub async fn connect(path: &str) -> Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let url = if path.starts_with('/') {
            format!("sqlite://{}", path)
        } else {
            format!("sqlite:{}", path)
        };
        let opts = SqliteConnectOptions::from_str(&url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_millis(5000));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;

        // ignore_missing: tolera migraciones que la DB ya tiene aplicadas pero
        // que ya no están en migrations/ (p.ej. la 033, que vino de una versión
        // previa del proyecto y se perdió al re-empaquetar desde el zip original).
        // Sin esto, sqlx larga "migration 33 was previously applied but is missing
        // in the resolved migrations" y el binario no arranca contra una DB con
        // historial. Las pending (en la carpeta, sin aplicar) sí se aplican
        // normalmente; el check de checksum para las que sí están sigue activo.
        let mut migrator = sqlx::migrate!("./migrations");
        migrator.set_ignore_missing(true);
        migrator.run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn ensure_owner(&self, username: &str, email: &str, password: &str) -> Result<()> {
        if password.is_empty() || password == "cambiar_este_password_ya" {
            return Err(anyhow!(
                "AETHRAVOX_OWNER_PASS no configurado. Edita .env antes de arrancar."
            ));
        }

        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = 'owner'")
                .fetch_optional(&self.pool)
                .await?;
        if existing.map(|(c,)| c > 0).unwrap_or(false) {
            return Ok(());
        }

        let hash = hash_password(password)?;
        sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, 'owner')",
        )
        .bind(username)
        .bind(email)
        .bind(hash)
        .execute(&self.pool)
        .await?;

        println!("👑 Owner '{}' creado correctamente.", username);
        println!("   ⚠️  Cambia la contraseña desde /dashboard después de iniciar sesión.");
        Ok(())
    }

    pub async fn create_user(&self, username: &str, email: &str, password: &str) -> Result<i64> {
        let hash = hash_password(password)?;
        let res = sqlx::query(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, 'usuario')",
        )
        .bind(username)
        .bind(email)
        .bind(hash)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, avatar_url, created_at, last_login_at, total_seconds, display_name, bio, status, last_ip, role_id, primary_title_id, visible_title_1, visible_title_2, visible_title_3 FROM users WHERE username = ? COLLATE NOCASE",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, avatar_url, created_at, last_login_at, total_seconds, display_name, bio, status, last_ip, role_id, primary_title_id, visible_title_1, visible_title_2, visible_title_3 FROM users WHERE email = ? COLLATE NOCASE",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn find_user_by_id(&self, id: i64) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, avatar_url, created_at, last_login_at, total_seconds, display_name, bio, status, last_ip, role_id, primary_title_id, visible_title_1, visible_title_2, visible_title_3 FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn update_password(&self, user_id: i64, new_password: &str) -> Result<()> {
        let hash = hash_password(new_password)?;
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn touch_last_login(&self, user_id: i64) -> Result<()> {
        sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, avatar_url, created_at, last_login_at, total_seconds, display_name, bio, status, last_ip, role_id, primary_title_id, visible_title_1, visible_title_2, visible_title_3 FROM users ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    pub async fn count_users_by_role(&self) -> Result<(i64, i64, i64, i64)> {
        let row: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                COALESCE(SUM(CASE WHEN role='owner'   THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN role='admin'   THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN role='staff'   THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN role='usuario' THEN 1 ELSE 0 END), 0)
             FROM users",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_user_role(&self, user_id: i64, new_role: &str) -> Result<()> {
        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(new_role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_profile(
        &self,
        user_id: i64,
        display_name: Option<&str>,
        bio: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET display_name = ?, bio = ?, avatar_url = COALESCE(?, avatar_url) WHERE id = ?",
        )
        .bind(display_name)
        .bind(bio)
        .bind(avatar_url)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_user_status(&self, user_id: i64, status: &str) -> Result<()> {
        sqlx::query("UPDATE users SET status = ? WHERE id = ?")
            .bind(status)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_user(&self, user_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("argon2 hash error: {}", e))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn random_hex(num_bytes: usize) -> String {
    let mut buf = vec![0u8; num_bytes];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

pub fn random_base32(n: usize) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut buf = vec![0u8; n];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| ALPHA[(*b as usize) % ALPHA.len()] as char).collect()
}

pub fn random_numeric(n: usize) -> String {
    let mut buf = vec![0u8; n];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| ((*b as usize) % 10).to_string()).collect()
}

mod sessions;
mod link;
mod recovery;
mod webs;
mod roles;
mod bans;
mod audit;
mod config;
mod documents;
mod chat;
mod friends;
mod ip_bans;
mod ranks;
mod communities;
mod categories;
mod support;
mod push;
mod titles;
pub mod emails; // Sección 6: user_emails + email_change_requests (UserEmail es público)
pub mod downloads; // Sección 7: tabla downloads (clientes nativos)
pub mod aris; // Aris: chats, miembros, mensajes, rate-limit y API keys
pub mod aexos; // Cuentas exclusivas y sesiones activas de aexOS
pub mod theme; // Sistema de temas y onboarding
pub mod analytics; // Carriles IA de Aris, analíticas de visitas y acciones sociales

pub mod zlibs; // Registro de librerías Zen (ZLibs + tokens CLI)
