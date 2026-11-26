-- Aethravox Studios — Esquema inicial v1
-- Web 1: Login/Register  +  tablas preparadas para Web 2 (hub) y Web 3 (dashboard).

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- ===== Usuarios =====
CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    username        TEXT UNIQUE NOT NULL COLLATE NOCASE,
    email           TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash   TEXT NOT NULL,
    role            TEXT NOT NULL DEFAULT 'usuario' CHECK (role IN ('owner', 'admin', 'staff', 'usuario')),
    avatar_url      TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_login_at   DATETIME,
    total_seconds   INTEGER NOT NULL DEFAULT 0  -- tiempo de uso acumulado (Web 3)
);

-- ===== Sesiones =====
CREATE TABLE IF NOT EXISTS sessions (
    token           TEXT PRIMARY KEY,
    user_id         INTEGER NOT NULL,
    expires_at      DATETIME NOT NULL,
    remember        INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_seen_at    DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sessions_user  ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_exp   ON sessions(expires_at);

-- ===== Tokens de recuperación de contraseña =====
CREATE TABLE IF NOT EXISTS recovery_tokens (
    token           TEXT PRIMARY KEY,
    user_id         INTEGER NOT NULL,
    expires_at      DATETIME NOT NULL,
    used            INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_recovery_user  ON recovery_tokens(user_id);

-- ===== Webs registradas en el Hub (Web 2) =====
CREATE TABLE IF NOT EXISTS webs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    slug            TEXT UNIQUE NOT NULL,        -- ej: "blog", "proyectos"
    name            TEXT NOT NULL,               -- ej: "Blog del Estudio"
    description     TEXT,
    icon            TEXT,                        -- emoji o clase CSS
    url             TEXT NOT NULL,               -- ej: "/web/blog"
    is_public       INTEGER NOT NULL DEFAULT 0,  -- 1 = visible sin login
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- ===== Permisos por rol sobre cada web =====
-- Permite que el Owner asigne qué roles ven qué webs.
CREATE TABLE IF NOT EXISTS web_role_perms (
    web_id          INTEGER NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'staff', 'usuario')),
    PRIMARY KEY (web_id, role),
    FOREIGN KEY (web_id) REFERENCES webs(id) ON DELETE CASCADE
);

-- ===== Logs de auditoría (admin) =====
CREATE TABLE IF NOT EXISTS audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER,
    action          TEXT NOT NULL,                -- login, logout, register, role_change, web_create...
    detail          TEXT,
    ip              TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(created_at);
