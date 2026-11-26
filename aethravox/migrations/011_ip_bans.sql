-- Aethravox Studios — Migración 011: Baneos de IP
-- Sistema para banear IPs que intenten ataques (DDoS, fuerza bruta, abuso).
-- El ban puede ser temporal (con expires_at) o permanente (expires_at NULL).
-- El middleware require_no_ip_ban bloquea TODO acceso desde IPs baneadas
-- y devuelve una pantalla de ban con countdown en tiempo real.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS ip_bans (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ip              TEXT NOT NULL,
    reason          TEXT NOT NULL,
    banned_by       INTEGER,                    -- user_id del admin que baneó (NULL si lo hizo el sistema)
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at      DATETIME,                    -- NULL = permanente
    revoked_at      DATETIME,                    -- NULL = activo
    FOREIGN KEY (banned_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_ip_bans_ip      ON ip_bans(ip);
CREATE INDEX IF NOT EXISTS idx_ip_bans_active  ON ip_bans(ip) WHERE revoked_at IS NULL;
