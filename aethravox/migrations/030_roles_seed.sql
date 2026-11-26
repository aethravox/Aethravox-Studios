-- Migración 030: poblar roles por defecto + sus permisos baseline.
--
-- Crea los 4 roles base del sistema con is_system=1 (no se pueden borrar)
-- y el owner con is_protected=1 (no se puede modificar name/role_key/
-- position). Luego pobla role_perms y role_web_perms con los permisos que
-- antes estaban hardcoded en baseline_has_perm.
--
-- Esto replica EXACTAMENTE el comportamiento anterior: el baseline de
-- `admin` sigue concediendo lo mismo que antes; el de `staff` idem; etc.
-- La diferencia es que ahora esos permisos viven en la DB y el admin puede
-- editarlos / crear roles nuevos con cualquier combinación.

-- ============================================================
-- 1) Crear los 4 roles base.
-- ============================================================
INSERT INTO roles (name, role_key, color, position, description, is_system, is_protected)
VALUES
    ('Owner',   'owner',   '#ef4444', 1000, 'Control total del sistema. No se puede modificar.', 1, 1),
    ('Admin',   'admin',   '#f5b301', 500,  'Administrador con acceso a casi todo el panel.',     1, 0),
    ('Staff',   'staff',   '#3b82f6', 200,  'Equipo de soporte con permisos limitados.',          1, 0),
    ('Usuario', 'usuario', '#9a9a9a', 100,  'Rol base. Sin acceso al panel salvo permiso extra.', 1, 0);

-- ============================================================
-- 2) Permisos atómicos por rol.
--    (replica baseline_has_perm de models.rs)
-- ============================================================

-- Owner: en runtime user_has_perm le hace bypass total (no lee role_perms).
-- Insertamos igual TODOS los permisos del catálogo para que la UI de
-- /admin/roles los muestre marcados (efecto cosmético; el motor no los lee).
INSERT INTO role_perms (role_id, perm)
SELECT r.id, p.perm FROM (
    SELECT 'admin.access'         AS perm UNION ALL
    SELECT 'users.view'            UNION ALL
    SELECT 'users.create'          UNION ALL
    SELECT 'users.role.change'     UNION ALL
    SELECT 'users.delete'          UNION ALL
    SELECT 'users.force_logout'    UNION ALL
    SELECT 'users.ban.temporary'  UNION ALL
    SELECT 'users.ban.permanent'  UNION ALL
    SELECT 'users.suspend'        UNION ALL
    SELECT 'users.unban'          UNION ALL
    SELECT 'users.rank.assign'    UNION ALL
    SELECT 'users.ip.view'        UNION ALL
    SELECT 'webs.view'            UNION ALL
    SELECT 'webs.create'          UNION ALL
    SELECT 'webs.delete'          UNION ALL
    SELECT 'webs.toggle'          UNION ALL
    SELECT 'webs.html.edit'       UNION ALL
    SELECT 'webs.perms'           UNION ALL
    SELECT 'webs.perms.assign'    UNION ALL
    SELECT 'webs.tag'             UNION ALL
    SELECT 'ranks.manage'         UNION ALL
    SELECT 'roles.manage'         UNION ALL
    SELECT 'config.edit'          UNION ALL
    SELECT 'audit.view'           UNION ALL
    SELECT 'ip_bans.view'         UNION ALL
    SELECT 'ip_bans.create'       UNION ALL
    SELECT 'ip_bans.revoke'       UNION ALL
    SELECT 'tickets.view'         UNION ALL
    SELECT 'tickets.respond'      UNION ALL
    SELECT 'tickets.manage'       UNION ALL
    SELECT 'docs.view'            UNION ALL
    SELECT 'docs.create'          UNION ALL
    SELECT 'docs.edit'            UNION ALL
    SELECT 'docs.delete'          UNION ALL
    SELECT 'profile.view_suspended' UNION ALL
    SELECT 'chat.moderate'        UNION ALL
    SELECT 'communities.moderate'
) p
CROSS JOIN roles r
WHERE r.role_key = 'owner';

-- Admin: baseline completo (lo que podía hacer antes).
INSERT INTO role_perms (role_id, perm)
SELECT r.id, p.perm FROM (
    SELECT 'admin.access'         AS perm UNION ALL
    SELECT 'users.view'            UNION ALL
    SELECT 'users.create'          UNION ALL
    SELECT 'users.role.change'     UNION ALL
    SELECT 'users.delete'          UNION ALL
    SELECT 'users.force_logout'    UNION ALL
    SELECT 'users.ban.temporary'  UNION ALL
    SELECT 'users.ban.permanent'  UNION ALL
    SELECT 'users.suspend'        UNION ALL
    SELECT 'users.unban'          UNION ALL
    SELECT 'users.rank.assign'    UNION ALL
    SELECT 'users.ip.view'        UNION ALL
    SELECT 'webs.view'            UNION ALL
    SELECT 'webs.create'          UNION ALL
    SELECT 'webs.delete'          UNION ALL
    SELECT 'webs.toggle'          UNION ALL
    SELECT 'webs.html.edit'       UNION ALL
    SELECT 'webs.perms'           UNION ALL
    SELECT 'webs.tag'             UNION ALL
    SELECT 'ranks.manage'         UNION ALL
    SELECT 'roles.manage'         UNION ALL
    SELECT 'config.edit'          UNION ALL
    SELECT 'audit.view'           UNION ALL
    SELECT 'ip_bans.view'         UNION ALL
    SELECT 'ip_bans.create'       UNION ALL
    SELECT 'ip_bans.revoke'       UNION ALL
    SELECT 'tickets.view'         UNION ALL
    SELECT 'tickets.respond'      UNION ALL
    SELECT 'tickets.manage'       UNION ALL
    SELECT 'docs.view'            UNION ALL
    SELECT 'docs.create'          UNION ALL
    SELECT 'docs.edit'            UNION ALL
    SELECT 'docs.delete'          UNION ALL
    SELECT 'profile.view_suspended' UNION ALL
    SELECT 'chat.moderate'        UNION ALL
    SELECT 'communities.moderate'
) p
CROSS JOIN roles r
WHERE r.role_key = 'admin';

-- Staff: baseline mínimo (lo que podía hacer antes).
INSERT INTO role_perms (role_id, perm)
SELECT r.id, p.perm FROM (
    SELECT 'admin.access'        AS perm UNION ALL
    SELECT 'users.ban.temporary' UNION ALL
    SELECT 'users.suspend'       UNION ALL
    SELECT 'tickets.view'        UNION ALL
    SELECT 'tickets.respond'     UNION ALL
    SELECT 'tickets.manage'      UNION ALL
    SELECT 'chat.moderate'       UNION ALL
    SELECT 'communities.moderate'
) p
CROSS JOIN roles r
WHERE r.role_key = 'staff';

-- Usuario: sin permisos del panel (no insertamos nada).
-- Los permisos extra los da un rango (rank_perms), no el rol.

-- ============================================================
-- 3) Visibilidad de secciones por rol (role_web_perms).
--    Replicamos lo que estaba en web_role_perms para owner/admin/staff
--    y lo extendemos a Usuario (ve las webs base: hub, dashboard, login).
-- ============================================================

-- Owner ve TODO (bypass en user_has_perm + web visible check).
-- Insertamos las webs base para que aparezcan marcadas en el admin.
INSERT INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE r.role_key = 'owner';

-- Admin: ve todo.
INSERT INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE r.role_key = 'admin';

-- Staff: ve las webs "base" (hub, dashboard, login) + soporte + comunidades.
-- (Replica lo que tenía web_role_perms antes.)
INSERT INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE r.role_key = 'staff'
  AND w.slug IN ('hub', 'dashboard', 'login', 'soporte', 'communities', 'chat', 'friends');

-- Usuario: solo las webs base (las que ya ve un usuario común logueado).
INSERT INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE r.role_key = 'usuario'
  AND w.slug IN ('hub', 'dashboard', 'login', 'soporte', 'communities', 'chat', 'friends', 'playground', 'transmitir');
