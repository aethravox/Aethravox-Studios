-- Migración 034: ajuste de permisos.
--
-- 1) Retirar `roles.manage` del rol Admin (y de cualquier rol no-owner que
--    lo tuviera). El panel de edición de roles queda SOLO para el Owner:
--    era un bug de seguridad que cualquier Admin pudiera reconfigurar
--    permisos del sistema. El Owner sigue teniendo bypass total.
--
-- 2) Conceder `titles.manage` SOLO al Owner. Es el permiso para crear,
--    editar, borrar títulos y asignarlos a usuarios. Nadie más lo tiene
--    por defecto (si un día se quiere delegar, se concede desde /admin/roles
--    —que ahora solo abre el Owner—).
--
-- 3) Añadir `titles.manage` al catálogo PERMISSIONS (en models.rs) para que
--    aparezca en el panel de roles. El Owner ya lo tiene vía bypass, pero
--    lo insertamos en role_perms para que la UI lo muestre marcado.

-- 1) Quitar roles.manage de TODOS los roles menos Owner.
DELETE FROM role_perms
WHERE perm = 'roles.manage'
  AND role_id NOT IN (SELECT id FROM roles WHERE role_key = 'owner');

-- 2) titles.manage solo al Owner (si por error lo tenía otro rol, lo quitamos).
DELETE FROM role_perms
WHERE perm = 'titles.manage'
  AND role_id NOT IN (SELECT id FROM roles WHERE role_key = 'owner');

-- Y se lo aseguramos al Owner (INSERT OR IGNORE por si ya existía).
INSERT OR IGNORE INTO role_perms (role_id, perm)
SELECT id, 'titles.manage' FROM roles WHERE role_key = 'owner';
