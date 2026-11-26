-- Aethravox Studios — Migración 010: Tags con mensaje + fecha de apertura
-- Permite que el admin, al marcar una web como "próximamente" o "mantenimiento",
-- añada un mensaje personalizado y/o una fecha/hora de apertura.
-- Cuando un usuario haga click en esa card en el hub, verá un modal explicando.
-- Idempotente: usa IF NOT EXISTS en ALTER (compatible con SQLite 3.35+).

-- tag_message: mensaje custom opcional. Si es NULL, se usa un default según el tag.
-- tag_opens_at: fecha/hora (UTC) opcional en la que la sección volverá a estar disponible.

-- SQLite no soporta ADD COLUMN IF NOT EXISTS hasta 3.35, y sqlx requiere SQLite 3.42+,
-- así que un ALTER directo fallaría si la columna ya existe. Lo protegemos con un
-- pragma check en Rust (db.rs) — aquí solo declaramos los ALTER esperados.

ALTER TABLE webs ADD COLUMN tag_message TEXT;
ALTER TABLE webs ADD COLUMN tag_opens_at DATETIME;
