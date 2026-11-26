-- Aethravox Studios — Migración 004: Documents (Web 4: Docs)
-- Galería de PPTX/DOCX/PDF. Solo el Owner sube/edita/elimina.
-- Todos los logueados pueden ver y descargar.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS documents (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    title           TEXT NOT NULL,
    description     TEXT,
    file_name       TEXT NOT NULL,             -- nombre original del archivo
    file_path       TEXT NOT NULL,             -- ruta relativa en data/files/
    file_type       TEXT NOT NULL,             -- 'pdf' | 'docx' | 'pptx' | 'unknown'
    file_size       INTEGER NOT NULL,          -- bytes
    preview_path    TEXT,                      -- ruta al PDF generado para preview (si aplica)
    preview_status  TEXT NOT NULL DEFAULT 'pending'
                    CHECK (preview_status IN ('pending', 'converting', 'ready', 'failed')),
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    downloaded      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_docs_type   ON documents(file_type);
CREATE INDEX IF NOT EXISTS idx_docs_created ON documents(created_at);

-- ===== Insertar la web "docs" en el hub =====
-- Visible para TODOS los roles logueados.
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('docs', 'Documentos', 'Galería de PPTX, DOCX y PDF. Solo lectura y descarga.', '📄',  '/docs',  0);

-- Permisos: todos los roles logueados pueden verla.
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'owner'    FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'admin'    FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'staff'    FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'usuario'  FROM webs WHERE slug = 'docs';
