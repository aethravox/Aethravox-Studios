// Sección 4 — Categorías del hub.
// Una categoría agrupa "apps" (webs) del hub en tiras horizontales.
// Modelo: categories (id, name, slug, position, created_at).
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// slug URL-safe único: /categoria/:slug.
    pub slug: String,
    /// Orden de aparición en el hub (menor = más arriba).
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
