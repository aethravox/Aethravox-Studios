-- Aethravox Studios — Migración 026: Rangos personalizados + permisos por rango
--
-- Permite al Owner crear rangos personalizados (p.ej. "Moderador", "VIP",
-- "Editor") y asignar a cada rango qué secciones (webs) puede ver en el hub.
-- Los usuarios pueden tener un rango asignado (además de su rol del sistema
-- owner/admin/staff/usuario). El acceso efectivo a una sección es la unión:
--   (permisos del rol en web_role_perms)  OR  (permisos del rango en rank_web_perms)
-- El Owner (rol del sistema) tiene bypass total: ve todas las secciones.
--
-- Tablas:
--   ranks            — definición de cada rango (nombre, color).
--   rank_web_perms   — qué webs puede ver cada rango (rank_id, web_id).
--   users.rank_id    — rango asignado a cada usuario (nullable).

CREATE TABLE IF NOT EXISTS ranks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    color      TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS rank_web_perms (
    rank_id INTEGER NOT NULL,
    web_id  INTEGER NOT NULL,
    UNIQUE(rank_id, web_id)
);

-- Añade la columna rank_id a users. SQLite >= 3.35 no necesita IF NOT EXISTS
-- en ALTER; sqlx migrate no re-aplica migraciones ya corridas, así que un
-- ALTER plano es seguro.
ALTER TABLE users ADD COLUMN rank_id INTEGER;
