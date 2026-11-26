-- Aethravox Studios — Migración 012: Historial de IPs por usuario
-- Registra TODAS las IPs desde las que cada usuario ha hecho login.
-- Permite detectar logins sospechosos (RD un día, EE.UU. al siguiente).
-- Cada IP se registra una vez por usuario y se actualiza last_seen + times_used.
-- Para mostrar rápido la última IP en el listado de admin, añadimos users.last_ip.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS user_ip_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    ip          TEXT NOT NULL,
    first_seen  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_seen   DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    times_used  INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- No puede haber dos filas con la misma (user_id, ip) — INSERT OR IGNORE evita duplicados.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_user_ip ON user_ip_history(user_id, ip);
CREATE INDEX IF NOT EXISTS idx_user_ip_history_user ON user_ip_history(user_id);

-- Columna para acceso rápido en /admin/users (sin JOIN).
ALTER TABLE users ADD COLUMN last_ip TEXT;
