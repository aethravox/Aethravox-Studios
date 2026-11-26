//! Queries de Sección 6: correos internos verificados (user_emails) y
//! solicitudes de verificación (email_change_requests).
//!
//! El reset de contraseña (6.4) reusa `recovery_tokens` (db/recovery.rs).

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct UserEmail {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    /// "YYYY-MM-DD HH:MM:SS" UTC o NULL si no verificado.
    pub verified_at: Option<String>,
    pub is_primary: i64,
    pub created_at: String,
}

impl UserEmail {
    pub fn is_verified(&self) -> bool {
        self.verified_at.is_some()
    }
}

impl crate::db::Db {
    /// Lista los correos internos vinculados a un usuario.
    pub async fn list_user_emails(&self, user_id: i64) -> Result<Vec<UserEmail>> {
        let rows = sqlx::query_as::<_, UserEmail>(
            "SELECT id, user_id, email, verified_at, is_primary, created_at \
             FROM user_emails WHERE user_id = ? ORDER BY is_primary DESC, created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Devuelve el correo interno verificado primario del usuario (si tiene).
    /// Se usa en 6.4 para saber a dónde mandar el enlace de reset.
    pub async fn primary_verified_email(&self, user_id: i64) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT email FROM user_emails \
             WHERE user_id = ? AND verified_at IS NOT NULL AND is_primary = 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// ¿Existe ya un correo interno vinculado (verificado o no) a un usuario?
    pub async fn user_email_exists(&self, user_id: i64, email: &str) -> Result<bool> {
        let n: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_emails WHERE user_id = ? AND email = ?",
        )
        .bind(user_id)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(n.0 > 0)
    }

    /// Inserta un correo verificado y lo marca primario (desmarcando los
    /// anteriores). Se usa tras consumir el código de verificación (6.2).
    pub async fn add_verified_user_email(&self, user_id: i64, email: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE user_emails SET is_primary = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        // upsert: si ya existe (no verificado), lo marcamos verificado+primary.
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, verified_at, is_primary) \
             VALUES (?, ?, datetime('now'), 1) \
             ON CONFLICT(user_id, email) DO UPDATE SET \
                verified_at = datetime('now'), is_primary = 1",
        )
        .bind(user_id)
        .bind(email)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Crea una solicitud de verificación de correo con su código de 8 dígitos.
    pub async fn create_email_change_request(
        &self,
        user_id: i64,
        new_email: &str,
        code: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_change_requests (user_id, new_email, code, expires_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(new_email)
        .bind(code)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Busca una solicitud de verificación válida (no usada, no expirada) para
    /// el usuario + código. Devuelve (id, new_email).
    pub async fn find_valid_email_change(
        &self,
        user_id: i64,
        code: &str,
    ) -> Result<Option<(i64, String)>> {
        let row: Option<(i64, String)> = sqlx::query_as(
            "SELECT id, new_email FROM email_change_requests \
             WHERE user_id = ? AND code = ? AND used_at IS NULL AND expires_at > ?",
        )
        .bind(user_id)
        .bind(code)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Marca una solicitud como usada (tras verificarla). Devuelve true si se
    /// consumió (existía y estaba válida).
    pub async fn consume_email_change(&self, request_id: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE email_change_requests SET used_at = datetime('now') \
             WHERE id = ? AND used_at IS NULL",
        )
        .bind(request_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Pone un correo como primario (desmarcando el resto del usuario).
    pub async fn set_primary_user_email(&self, user_id: i64, email_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE user_emails SET is_primary = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE user_emails SET is_primary = 1 WHERE id = ? AND user_id = ?")
            .bind(email_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Elimina un correo vinculado (solo si no es el primario verificado).
    pub async fn remove_user_email(&self, user_id: i64, email_id: i64) -> Result<bool> {
        let res = sqlx::query(
            "DELETE FROM user_emails WHERE id = ? AND user_id = ? AND is_primary = 0",
        )
        .bind(email_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Inserta un correo NO verificado como primario (desmarca los anteriores).
    /// Sección 6.3: al registrarse, el usuario da un correo interno que queda
    /// vinculado pero pendiente de verificación (la completa después en 6.2).
    /// Upsert: si ya existía, lo deja como primario (sin tocar verified_at).
    pub async fn add_unverified_user_email(&self, user_id: i64, email: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE user_emails SET is_primary = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, verified_at, is_primary) \
             VALUES (?, ?, NULL, 1) \
             ON CONFLICT(user_id, email) DO UPDATE SET is_primary = 1",
        )
        .bind(user_id)
        .bind(email)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// ¿El correo del usuario está verificado? (verified_at IS NOT NULL).
    pub async fn user_email_is_verified(&self, user_id: i64, email: &str) -> Result<bool> {
        let n: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_emails \
             WHERE user_id = ? AND email = ? AND verified_at IS NOT NULL",
        )
        .bind(user_id)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(n.0 > 0)
    }

    // ── helpers para tests/interna ──────────────────────────────────────
    #[allow(dead_code)]
    pub async fn _count_user_emails(&self, user_id: i64) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_emails WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    /// Pool de sólo lectura (para reuso interno si hace falta).
    #[allow(dead_code)]
    pub fn _pool(&self) -> &SqlitePool {
        &self.pool
    }
}
