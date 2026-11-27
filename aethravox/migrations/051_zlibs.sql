-- Aethravox Studios — Migración 051: ZLibs (Registro de librerías Zen + Tokens CLI)

CREATE TABLE IF NOT EXISTS zen_cli_tokens (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label        TEXT NOT NULL DEFAULT 'Zen CLI',
    token_hash   TEXT NOT NULL UNIQUE,
    token_prefix TEXT NOT NULL DEFAULT '',
    created_at   DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_used_at DATETIME
);

CREATE INDEX IF NOT EXISTS idx_zen_cli_tokens_user ON zen_cli_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_zen_cli_tokens_hash ON zen_cli_tokens(token_hash);

CREATE TABLE IF NOT EXISTS zen_packages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    owner_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    description     TEXT NOT NULL DEFAULT '',
    git_url         TEXT NOT NULL,
    readme          TEXT NOT NULL DEFAULT '',
    latest_version  TEXT NOT NULL DEFAULT '',
    downloads_count INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_zen_packages_owner ON zen_packages(owner_id);
CREATE INDEX IF NOT EXISTS idx_zen_packages_updated ON zen_packages(updated_at DESC);

CREATE TABLE IF NOT EXISTS zen_package_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id  INTEGER NOT NULL REFERENCES zen_packages(id) ON DELETE CASCADE,
    version     TEXT NOT NULL,
    git_ref     TEXT NOT NULL,
    manifest    TEXT NOT NULL DEFAULT '',
    files_count INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(package_id, version)
);

CREATE INDEX IF NOT EXISTS idx_zen_pkg_versions_pkg ON zen_package_versions(package_id, id DESC);

-- Registrar ZLibs en el Hub (pública y visible para todos, incluido invitado).
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public, is_active)
VALUES (
    'zlibs',
    'ZLibs',
    'Registro oficial de librerías para Zen con repositorios en GitHub, tokens CLI y dependencias Make.mz.',
    '📦',
    '/zlibs',
    1,
    1
);

INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT w.id, r.role
FROM webs w
CROSS JOIN (
    SELECT 'owner' AS role
    UNION ALL SELECT 'admin'
    UNION ALL SELECT 'staff'
    UNION ALL SELECT 'usuario'
) r
WHERE w.slug = 'zlibs';

INSERT OR IGNORE INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE w.slug = 'zlibs'
  AND r.role_key IN ('owner', 'admin', 'staff', 'usuario', 'invitado');

INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id
FROM webs w
JOIN categories c ON w.slug = 'zlibs' AND c.slug = 'herramientas';
