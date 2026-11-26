-- Aethravox Studios — Migración 014: Secciones activables + HTML custom
-- Permite:
-- 1. Desactivar/activar cualquier sección (system o custom). Al desactivar, la card
--    desaparece del hub. La ruta hardcoded sigue funcionando si existe.
-- 2. Crear secciones custom con HTML propio que se sirve en /slug (para probar
--    webs antes de programarlas en Rust).
-- Idempotente: ALTER TABLE falla si la columna ya existe, pero sqlx::migrate!()
-- lo gestiona con su tabla _sqlx_migrations (no re-corre migraciones ya aplicadas).

PRAGMA foreign_keys = ON;

-- is_active: 1 = visible en el hub, 0 = oculta (desactivada).
ALTER TABLE webs ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

-- is_custom: 1 = sección custom creada por el owner (se puede eliminar),
--            0 = sección del sistema (no se puede eliminar, solo desactivar).
ALTER TABLE webs ADD COLUMN is_custom INTEGER NOT NULL DEFAULT 0;

-- html_content: NULL para secciones que usan URL externa o ruta hardcoded.
-- Si está setado, el fallback handler lo sirve como text/html en /slug.
ALTER TABLE webs ADD COLUMN html_content TEXT;
