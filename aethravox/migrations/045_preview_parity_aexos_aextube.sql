-- Aethravox Studios — Migración 045: Paridad con Preview (Aris pin, aexOS y aexTube)
--
-- 1) Añade columna `pinned` a `aris_chats` para fijar conversaciones arriba.
-- 2) Activa las secciones `aexos` y `aextube` quitándoles el tag 'Próximamente' y sincronizando role_web_perms.
-- 3) Crea las tablas de `aextube_videos`, `aextube_reactions` y `aextube_subscriptions`.

-- ============================================================
-- 1) Chats fijados en Aris
-- ============================================================
ALTER TABLE aris_chats ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;

-- ============================================================
-- 2) Activar aexOS y aexTube en el Hub
-- ============================================================
UPDATE webs
SET tag = '',
    description = 'Entorno de escritorio web interactivo con ventanas, terminal, notas y acceso integrado al ecosistema.'
WHERE slug = 'aexos' AND tag = 'Próximamente';

UPDATE webs
SET tag = '',
    description = 'Plataforma de vídeo del estudio con subida de clips (hasta 20 MB), Shorts, suscripciones y reacciones.'
WHERE slug = 'aextube' AND tag = 'Próximamente';

INSERT OR IGNORE INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
JOIN webs w ON w.slug IN ('aexos', 'aextube')
WHERE r.role_key IN ('owner', 'admin', 'staff', 'usuario');

-- ============================================================
-- 3) Tablas para aexTube (vídeos locales, reacciones y canales)
-- ============================================================
CREATE TABLE IF NOT EXISTS aextube_videos (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    author      TEXT NOT NULL,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    is_short    INTEGER NOT NULL DEFAULT 0,
    file_path   TEXT NOT NULL,
    mime_type   TEXT NOT NULL DEFAULT 'video/mp4',
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    likes       INTEGER NOT NULL DEFAULT 0,
    dislikes    INTEGER NOT NULL DEFAULT 0,
    views       INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_aextube_videos_created ON aextube_videos(created_at DESC);

CREATE TABLE IF NOT EXISTS aextube_reactions (
    video_id    INTEGER NOT NULL,
    user_id     INTEGER NOT NULL,
    reaction    TEXT NOT NULL CHECK(reaction IN ('like', 'dislike')),
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (video_id, user_id),
    FOREIGN KEY (video_id) REFERENCES aextube_videos(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id)  REFERENCES users(id)          ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS aextube_subscriptions (
    channel_author TEXT NOT NULL,
    user_id        INTEGER NOT NULL,
    created_at     DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (channel_author, user_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
