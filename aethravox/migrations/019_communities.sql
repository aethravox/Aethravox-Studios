-- Aethravox Studios — Migración 019: Comunidades (Fase 5)
-- Las comunidades agrupan conversaciones (grupos) bajo un mismo espacio.
-- Estilo Discord: una comunidad tiene varios grupos dentro.
-- Para entrar a un grupo de una comunidad, necesitas ser miembro de la comunidad.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS communities (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    description     TEXT,
    icon            TEXT,                        -- emoji
    image_url       TEXT,                        -- URL de imagen (como grupos)
    owner_id        INTEGER NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    is_active       INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS community_members (
    community_id    INTEGER NOT NULL,
    user_id         INTEGER NOT NULL,
    joined_at       DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (community_id, user_id),
    FOREIGN KEY (community_id) REFERENCES communities(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Vincular conversaciones a comunidades (NULL = conversación independiente)
ALTER TABLE conversations ADD COLUMN community_id INTEGER REFERENCES communities(id) ON DELETE SET NULL;

-- Registrar la web "communities" en el hub
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('communities', 'Comunidades', 'Espacios con múltiples grupos organizados.', '🏘️', '/communities', 0);

INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'communities';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'communities';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'communities';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'communities';
