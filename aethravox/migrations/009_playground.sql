-- Aethravox Studios — Migración 009: Playground (Web 6: Editor de código)
-- Editor de HTML/CSS/JS con vista previa en vivo y descarga.
-- Pensado para usuarios que quieren prototipar snippets sin montar un entorno local.
-- Totalmente client-side (no requiere nada del backend excepto servir la página).

-- Idempotente: seguro de correr en DB nueva o existente.
PRAGMA foreign_keys = ON;

-- ===== Registrar la web "playground" en el hub =====
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('playground', 'Playground',
     'Editor de HTML, CSS y JS con vista previa en vivo. Descarga tu código cuando quieras.',
     '🛠️', '/playground', 0);

-- Permisos: TODOS los roles logueados pueden usar el playground.
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'owner'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'admin'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'staff'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT id, 'usuario' FROM webs WHERE slug = 'playground';
