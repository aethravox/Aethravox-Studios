-- Aethravox Studios — Migración 013: Restaurar secciones del sistema
-- Si un owner (por error o por cuenta comprometida) borró secciones del sistema
-- (login, hub, dashboard, chat, friends, playground, docs), esta migración las
-- restaura. Es IDEMPOTENTE: usa INSERT OR IGNORE, así que no duplica filas
-- si ya existen.
--
-- IMPORTANTE: las rutas /login, /hub, /dashboard, /chat, /friends, /playground,
-- /docs SIGUEN funcionando aunque las filas en `webs` no existan, porque las
-- rutas están hardcoded en main.rs. Pero el hub NO las muestra como cards sin
-- estas filas, y /admin/webs no las lista. Esta migración las restaura.

PRAGMA foreign_keys = ON;

-- ===== Insertar todas las secciones del sistema (INSERT OR IGNORE = no duplica) =====
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('hub',        'Panel principal',  'Acceso a todas las secciones del estudio.',  '🏛️',  '/hub',        0),
    ('dashboard',  'Mi perfil',        'Datos de cuenta, avatar, sesiones activas.', '🛡️',  '/dashboard',  0),
    ('login',      'Acceso',            'Iniciar sesión o crear cuenta.',             '⚔️',  '/',           1),
    ('chat',       'Mensajería',        'Chat privado y grupal en tiempo real.',      '💬',  '/chat',       0),
    ('friends',    'Amigos',            'Lista de amigos y solicitudes pendientes.',   '👥',  '/friends',    0),
    ('playground', 'Playground',        'Editor de HTML, CSS y JS con vista previa en vivo. Descarga tu código cuando quieras.', '🛠️', '/playground', 0),
    ('docs',       'Documentos',        'Galería de PPTX, DOCX y PDF. Solo lectura y descarga.', '📄', '/docs', 0);

-- ===== Permisos por rol para cada sección del sistema =====
-- (INSERT OR IGNORE + SELECT id FROM webs WHERE slug = ... evita duplicados y
--  no falla si la sección aún no existiera.)

-- Hub: owner + admin + staff (no visible para 'usuario' porque es el panel de admin)
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'hub';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'hub';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'hub';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'hub';

-- Dashboard: owner + admin + staff + usuario (todos los logueados pueden ver su perfil)
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'dashboard';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'dashboard';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'dashboard';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'dashboard';

-- Login: owner + admin + usuario (staff no necesita ver el login, ya está logueado)
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'login';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'login';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'login';

-- Chat: owner + admin + staff + usuario (todos los logueados pueden chatear)
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'chat';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'chat';

-- Friends: owner + admin + staff + usuario
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'friends';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'friends';

-- Playground: owner + admin + staff + usuario
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'playground';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'playground';

-- Docs: owner + admin + staff + usuario
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'docs';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'docs';
