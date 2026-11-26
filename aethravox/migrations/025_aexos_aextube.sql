-- Aethravox Studios — Migración 025: Secciones aexOS y aexTube (Próximamente)
--
-- Añade dos nuevas secciones al hub marcadas como "Próximamente":
--   * aexOS   — sistema operativo web del estudio.
--   * aexTube — plataforma de vídeo del estudio.
--
-- Como llevan tag='Próximamente', la card del hub NO es clickable
-- (ver Web::is_clickable en models.rs): al hacer click se abre el modal
-- explicativo "🚧 Esta sección está en construcción. Pronto estará disponible."
-- Cuando el owner quiera activarlas de verdad, basta con que edite la sección
-- desde /admin/webs y le quite el tag (o lo cambie a 'nuevo' / 'beta').
--
-- IDEMPOTENTE: usa INSERT OR IGNORE, así que no duplica filas si ya existen.

-- ===== Filas en webs =====
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public, tag)
VALUES
    ('aexos',   'aexOS',   'El sistema operativo web de Aethravox Studios.', '🖥️', '/aexos',   0, 'Próximamente'),
    ('aextube', 'aexTube', 'Plataforma de vídeo para tus creaciones y directos.', '📺', '/aextube', 0, 'Próximamente');

-- ===== Permisos por rol: visibles para todos los roles logueados =====
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'aexos';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'aexos';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'aexos';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'aexos';

INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'aextube';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'aextube';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'aextube';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'aextube';
