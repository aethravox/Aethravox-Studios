// Sección 4 — DB methods para categorías del hub.
#![allow(unused_imports)]
use anyhow::Result;

use crate::models::{Category, Web};

use super::Db;

impl Db {
    /// Lista todas las categorías ordenadas por posición (menor = más arriba).
    pub async fn list_categories(&self) -> Result<Vec<Category>> {
        let rows = sqlx::query_as::<_, Category>(
            "SELECT id, name, slug, position, created_at FROM categories ORDER BY position ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Categoría por slug (para /categoria/:slug).
    pub async fn get_category_by_slug(&self, slug: &str) -> Result<Option<Category>> {
        let row = sqlx::query_as::<_, Category>(
            "SELECT id, name, slug, position, created_at FROM categories WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Devuelve pares (app_id, category_id) de app_categories. Lo usa el hub
    /// para agrupar las apps visibles del usuario por categoría en Rust. El
    /// filtrado de visibilidad por rol lo hace list_webs_for_role aparte.
    pub async fn list_app_category_pairs(&self) -> Result<Vec<(i64, i64)>> {
        let rows: Vec<(i64, i64)> =
            sqlx::query_as("SELECT app_id, category_id FROM app_categories")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    /// Apps (webs activas, no ocultas) en una categoría, ordenadas por id.
    /// Sin filtro de rol — el handler intersecta con las visibles para el viewer.
    /// Excluye hub/login/dashboard (no son "apps", están ocultas del hub).
    pub async fn list_apps_in_category(&self, category_id: i64) -> Result<Vec<Web>> {
        let rows = sqlx::query_as::<_, Web>(
            "SELECT w.id, w.slug, w.name, w.description, w.icon, w.url, w.is_public, w.created_at, w.tag, w.tag_message, w.tag_opens_at, w.is_active, w.is_custom, w.html_content
             FROM webs w
             JOIN app_categories ac ON ac.app_id = w.id
             WHERE ac.category_id = ? AND w.is_active = 1
               AND w.slug NOT IN ('hub', 'login', 'dashboard', 'correo')
             ORDER BY w.id ASC",
        )
        .bind(category_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Todas las apps activas no ocultas (para /explorar — sin filtro de rol).
    pub async fn list_all_active_webs(&self) -> Result<Vec<Web>> {
        let rows = sqlx::query_as::<_, Web>(
            "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
             FROM webs WHERE is_active = 1
               AND slug NOT IN ('hub', 'login', 'dashboard', 'correo')
             ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Busca apps activas por nombre (LIKE, case-insensitive vía SQLite NOCASE
    /// implícito en LIKE sobre TEXT sin COLLATE). Sin filtro de rol.
    pub async fn search_webs(&self, q: &str) -> Result<Vec<Web>> {
        let pattern = format!("%{}%", q.trim());
        let rows = sqlx::query_as::<_, Web>(
            "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
             FROM webs WHERE is_active = 1
               AND slug NOT IN ('hub', 'login', 'dashboard', 'correo')
               AND name LIKE ?
             ORDER BY id ASC",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Cuenta de apps activas por categoría (para /categorias). LEFT JOIN para
    /// que las categorías sin apps también aparezcan (count 0).
    /// Devuelve (category_id, count).
    pub async fn count_apps_per_category(&self) -> Result<Vec<(i64, i64)>> {
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT c.id, COUNT(w.id)
             FROM categories c
             LEFT JOIN app_categories ac ON ac.category_id = c.id
             LEFT JOIN webs w ON w.id = ac.app_id AND w.is_active = 1
                                AND w.slug NOT IN ('hub', 'login', 'dashboard', 'correo')
             GROUP BY c.id
             ORDER BY c.position ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
