-- Aethravox Studios — Migración 002: Web 2 + Web 3
-- Añade soporte para bans, suspensiones, reportes de contenido,
-- moderación, configuración global y tracking de sesiones por dispositivo.

PRAGMA foreign_keys = ON;

-- ===== Baneos permanentes =====
CREATE TABLE IF NOT EXISTS user_bans (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL,
    banned_by       INTEGER NOT NULL,
    reason          TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at      DATETIME,
    revoked_at      DATETIME,
    FOREIGN KEY (user_id)     REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (banned_by)   REFERENCES users(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_bans_user     ON user_bans(user_id);
CREATE INDEX IF NOT EXISTS idx_bans_active   ON user_bans(user_id) WHERE revoked_at IS NULL;

-- ===== Suspensiones temporales =====
CREATE TABLE IF NOT EXISTS user_suspensions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL,
    suspended_by    INTEGER NOT NULL,
    reason          TEXT,
    expires_at      DATETIME NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    revoked_at      DATETIME,
    FOREIGN KEY (user_id)      REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (suspended_by) REFERENCES users(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_susp_user     ON user_suspensions(user_id);
CREATE INDEX IF NOT EXISTS idx_susp_active   ON user_suspensions(user_id) WHERE revoked_at IS NULL;

-- ===== Reportes de contenido =====
CREATE TABLE IF NOT EXISTS content_reports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    reporter_id     INTEGER NOT NULL,
    target_type     TEXT NOT NULL,
    target_id       INTEGER NOT NULL,
    reason          TEXT NOT NULL,
    detail          TEXT,
    status          TEXT NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'reviewing', 'resolved', 'dismissed')),
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    resolved_by     INTEGER,
    resolved_at     DATETIME,
    FOREIGN KEY (reporter_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (resolved_by)  REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_reports_status ON content_reports(status);
CREATE INDEX IF NOT EXISTS idx_reports_target ON content_reports(target_type, target_id);

-- ===== Acciones de moderación =====
CREATE TABLE IF NOT EXISTS content_moderation (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    moderator_id    INTEGER NOT NULL,
    target_type     TEXT NOT NULL,
    target_id       INTEGER NOT NULL,
    action          TEXT NOT NULL,
    reason          TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (moderator_id) REFERENCES users(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_mod_target ON content_moderation(target_type, target_id);

-- ===== Configuración global =====
CREATE TABLE IF NOT EXISTS system_config (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    updated_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_by      INTEGER,
    FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE SET NULL
);

INSERT OR IGNORE INTO system_config (key, value) VALUES
    ('studio_name',        'Aethravox Studios'),
    ('foundation_date',    '2026-01-24'),
    ('allow_public_register', '1'),
    ('default_role',       'usuario'),
    ('maintenance_mode',   '0');

-- ===== Extensiones a la tabla users =====
ALTER TABLE users ADD COLUMN display_name TEXT;
ALTER TABLE users ADD COLUMN bio TEXT;
ALTER TABLE users ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'suspended', 'banned'));

-- ===== Datos de ejemplo: webs iniciales =====
-- Las 3 webs base del sistema. El Owner puede añadir más desde /admin/webs.
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('hub',        'Panel principal',  'Acceso a todas las secciones del estudio.',  '🏛️',  '/hub',        0),
    ('dashboard',  'Mi perfil',        'Datos de cuenta, avatar, sesiones activas.', '🛡️',  '/dashboard',  0),
    ('login',      'Acceso',            'Iniciar sesión o crear cuenta.',             '⚔️',  '/',           1);

-- Permisos por defecto para las 3 webs base.
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'owner'    FROM webs WHERE slug IN ('hub', 'dashboard', 'login');
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'admin'    FROM webs WHERE slug IN ('hub', 'dashboard', 'login');
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'staff'    FROM webs WHERE slug IN ('hub', 'dashboard');
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'usuario'  FROM webs WHERE slug IN ('dashboard', 'login');
