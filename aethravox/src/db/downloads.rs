#![allow(unused_imports)]
// Sección 7 — Descargas nativas: métodos de DB para la tabla `downloads`.
// La tabla la crea la migración 040_downloads.sql. Los archivos físicos viven
// en data/downloads/ (servidos por ServeDir en /descargas/files).

use anyhow::Result;
use sqlx::FromRow;

use super::Db;

/// Una fila de la tabla `downloads`. Mapeada con sqlx::FromRow.
#[derive(FromRow, Debug, Clone)]
pub struct Download {
    pub id: i64,
    pub platform: String,
    pub app_slug: String,
    pub app_name: String,
    pub version: String,
    pub filename: String,
    pub orig_name: String,
    pub size_bytes: i64,
    pub checksum: Option<String>,
    pub notes: Option<String>,
    pub is_active: i64,
    pub created_at: String,
}

impl Db {
    /// Descargas activas para la página pública /descargas, ordenadas por
    /// plataforma y app para facilitar el agrupado en el handler.
    pub async fn list_active_downloads(&self) -> Result<Vec<Download>> {
        sqlx::query_as::<_, Download>(
            "SELECT id, platform, app_slug, app_name, version, filename, orig_name,
                    size_bytes, checksum, notes, is_active, created_at
             FROM downloads WHERE is_active = 1
             ORDER BY platform, app_slug, id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Todas las descargas (activas y no) para el panel admin.
    pub async fn list_all_downloads(&self) -> Result<Vec<Download>> {
        sqlx::query_as::<_, Download>(
            "SELECT id, platform, app_slug, app_name, version, filename, orig_name,
                    size_bytes, checksum, notes, is_active, created_at
             FROM downloads ORDER BY platform, app_slug, id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_download(
        &self,
        platform: &str,
        app_slug: &str,
        app_name: &str,
        version: &str,
        filename: &str,
        orig_name: &str,
        size_bytes: i64,
        notes: Option<&str>,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO downloads
                (platform, app_slug, app_name, version, filename, orig_name, size_bytes, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(platform)
        .bind(app_slug)
        .bind(app_name)
        .bind(version)
        .bind(filename)
        .bind(orig_name)
        .bind(size_bytes)
        .bind(notes)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn delete_download(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_download(&self, id: i64) -> Result<Option<Download>> {
        let row = sqlx::query_as::<_, Download>(
            "SELECT id, platform, app_slug, app_name, version, filename, orig_name,
                    size_bytes, checksum, notes, is_active, created_at
             FROM downloads WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}
