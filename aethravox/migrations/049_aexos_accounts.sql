-- Aethravox Studios — Migración 049: Cuentas exclusivas de aexOS (sin contraseña, ejecución aislada)
-- Cada perfil creado en aexOS es una fila real en `users` (role='usuario', password_hash='!aexos_only!')
-- vinculada a su creador en `aexos_accounts` para que solo pueda abrirse dentro de aexOS.

CREATE TABLE IF NOT EXISTS aexos_accounts (
    user_id            INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    owner_user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallpaper          TEXT NOT NULL DEFAULT 'horizon',
    show_desktop_icons INTEGER NOT NULL DEFAULT 1,
    pinned_apps_json   TEXT NOT NULL DEFAULT '["files","terminal","aris","chat","aextube"]',
    files_json         TEXT NOT NULL DEFAULT '[]',
    notes_text         TEXT NOT NULL DEFAULT '',
    created_at         DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_aexos_accounts_owner ON aexos_accounts(owner_user_id);

CREATE TABLE IF NOT EXISTS aexos_sessions (
    token           TEXT PRIMARY KEY,
    owner_user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    profile_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at      DATETIME NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_aexos_sessions_owner ON aexos_sessions(owner_user_id);
