-- Aethravox Studios — Migración 007: Chat (Web 2)
-- Sistema de mensajería estilo WhatsApp con WebSocket.

PRAGMA foreign_keys = ON;

-- ===== Conversaciones (DM o grupo) =====
CREATE TABLE IF NOT EXISTS conversations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    type            TEXT NOT NULL CHECK(type IN ('dm', 'group')),
    name            TEXT,               -- NULL para DM, nombre para grupo
    created_by      INTEGER NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

-- ===== Miembros de cada conversación =====
CREATE TABLE IF NOT EXISTS conversation_members (
    conversation_id INTEGER NOT NULL,
    user_id         INTEGER NOT NULL,
    joined_at       DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_read_at    DATETIME,
    PRIMARY KEY (conversation_id, user_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_cm_user ON conversation_members(user_id);
CREATE INDEX IF NOT EXISTS idx_cm_conv ON conversation_members(conversation_id);

-- ===== Mensajes =====
CREATE TABLE IF NOT EXISTS messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    sender_id       INTEGER NOT NULL,
    body            TEXT NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_msgs_conv ON messages(conversation_id, created_at);

-- ===== Registrar la web "chat" en el hub =====
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('chat', 'Mensajería', 'Chat privado y grupal en tiempo real.', '💬', '/chat', 0);

INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'owner'    FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'admin'    FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'staff'    FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'usuario'  FROM webs WHERE slug = 'chat';
