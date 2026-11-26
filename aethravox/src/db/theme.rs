//! Queries del sistema de temas.

use sqlx::SqlitePool;
use super::Db;

/// Tema actual del usuario (fila en user_themes). None = nunca eligió.
pub async fn get_user_theme(pool: &SqlitePool, user_id: i64) -> Option<String> {
    sqlx::query_as::<_, (String,)>("SELECT theme FROM user_themes WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
}

/// Guarda o actualiza el tema del usuario.
pub async fn set_user_theme(pool: &SqlitePool, user_id: i64, theme: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_themes (user_id, theme, updated_at)
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
         ON CONFLICT(user_id) DO UPDATE SET
             theme = excluded.theme, updated_at = excluded.updated_at"
    )
    .bind(user_id)
    .bind(theme)
    .execute(pool)
    .await?;
    Ok(())
}

impl Db {
    pub async fn get_user_theme(&self, user_id: i64) -> Option<String> {
        get_user_theme(&self.pool, user_id).await
    }

    pub async fn set_user_theme(&self, user_id: i64, theme: &str) -> Result<(), sqlx::Error> {
        set_user_theme(&self.pool, user_id, theme).await
    }
}
