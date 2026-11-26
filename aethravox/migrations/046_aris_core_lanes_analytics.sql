-- Aethravox Studios — Migración 046: Carriles IA de Aris en el núcleo + Analíticas reales de visitas
--
-- 1) Tabla `web_visits` para registrar visitas por sección/rol y responder consultas
--    de Aris ("resumen de webs más usadas en X días", "cuántas personas entraron a...").
-- 2) Siembra los 7 carriles `aris.lane.*` en `role_perms` para admin/staff/usuario/owner.

CREATE TABLE IF NOT EXISTS web_visits (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    slug       TEXT NOT NULL,
    user_id    INTEGER,
    role_key   TEXT NOT NULL DEFAULT 'invitado',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_web_visits_created ON web_visits(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_web_visits_slug_created ON web_visits(slug, created_at DESC);

-- Owner y Admin: todos los carriles IA de Aris activos por defecto
INSERT OR IGNORE INTO role_perms (role_id, perm)
SELECT r.id, p.perm
FROM roles r
CROSS JOIN (
    SELECT 'aris.lane.chat'         AS perm UNION ALL
    SELECT 'aris.lane.quick_drawer' UNION ALL
    SELECT 'aris.lane.playground'   UNION ALL
    SELECT 'aris.lane.studio'       UNION ALL
    SELECT 'aris.lane.social'       UNION ALL
    SELECT 'aris.lane.analytics'    UNION ALL
    SELECT 'aris.lane.admin_ops'
) p
WHERE r.role_key IN ('owner', 'admin');

-- Staff: chat, drawer rápido, estudio, social y analíticas (sin ops ni playground por defecto; Owner puede editar)
INSERT OR IGNORE INTO role_perms (role_id, perm)
SELECT r.id, p.perm
FROM roles r
CROSS JOIN (
    SELECT 'aris.lane.chat'         AS perm UNION ALL
    SELECT 'aris.lane.quick_drawer' UNION ALL
    SELECT 'aris.lane.studio'       UNION ALL
    SELECT 'aris.lane.social'       UNION ALL
    SELECT 'aris.lane.analytics'
) p
WHERE r.role_key = 'staff';

-- Usuario base: chat general y acciones sociales con amigos aceptados
INSERT OR IGNORE INTO role_perms (role_id, perm)
SELECT r.id, p.perm
FROM roles r
CROSS JOIN (
    SELECT 'aris.lane.chat'   AS perm UNION ALL
    SELECT 'aris.lane.social'
) p
WHERE r.role_key IN ('usuario', 'user');
