-- Aethravox Studios — Migración 008: Friends (Web 5: Amigos)
-- Sistema de amigos con solicitudes pendientes, aceptar/rechazar y eliminar.
-- Idempotente: seguro de correr en DB nueva o en DB existente con datos previos.

PRAGMA foreign_keys = ON;

-- ===== Tabla de amistades =====
-- status: 'pending' | 'accepted' | 'rejected'
-- Para una amistad aceptada existen 2 filas (A->B y B->A) bidireccionales;
-- para una solicitud pendiente solo existe una fila (solicitante -> receptor).
CREATE TABLE IF NOT EXISTS friends (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL,
    friend_id    INTEGER NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at   DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    accepted_at  DATETIME,
    FOREIGN KEY (user_id)   REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (friend_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_friends_user   ON friends(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_friend ON friends(friend_id);

-- ===== Evitar duplicados de solicitudes pendientes =====
-- Un usuario no puede enviar 2 solicitudes pendientes al mismo receptor.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_friends_pending
    ON friends(user_id, friend_id) WHERE status = 'pending';

-- ===== Registrar la web "friends" en el hub =====
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('friends', 'Amigos', 'Lista de amigos y solicitudes pendientes.', '👥', '/friends', 0);

-- Permisos: TODOS los roles logueados pueden ver la sección de amigos.
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'owner'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'admin'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'staff'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'usuario' FROM webs WHERE slug = 'friends';
