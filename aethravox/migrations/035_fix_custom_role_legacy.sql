-- Migración 035: reparar users.role para usuarios con un rol custom asignado.
--
-- ANTES de esta migración, assign_user_role escribía el role_key crudo del rol
-- asignado en users.role. Para roles del sistema (user/admin/staff/owner) eso
-- es correcto (es lo que esperan list_webs_for_role, baseline_has_perm y los
-- CSS role-*). Pero para roles CUSTOM (role_key = "custom_N") escribía
-- "custom_N" en users.role, lo que rompía:
--   - list_webs_for_role: JOIN web_role_perms WHERE role = 'custom_N' → 0 filas
--     → el usuario no veía ninguna web en el hub.
--   - baseline_has_perm('custom_N', ...): siempre false → el usuario perdía
--     todos los permisos base (no podía entrar a /admin, /hub vacío, etc.).
--   - user_has_perm solo llegaba al fallback baseline_has_perm si role_id era
--     None; con role_id puesto usaba role_has_perm(role_id), pero el bypass
--     de owner y la lógica de webs seguían leyendo users.role.
--
-- Esta migración recalcula users.role para los usuarios cuyo rol asignado es
-- custom, derivando el legacy equivalente según la posición del rol:
--   position >= 500 → 'admin'
--   position >= 200 → 'staff'
--   resto           → 'user'
--
-- Los roles del sistema (is_system=1, role_key IN user/admin/staff/owner) ya
-- tienen users.role correcto y no se tocan. Solo se actualizan los usuarios
-- cuyo role_id apunta a un rol custom.

UPDATE users
SET role = CASE
    WHEN r.position >= 500 THEN 'admin'
    WHEN r.position >= 200 THEN 'staff'
    ELSE 'user'
END
FROM roles r
WHERE users.role_id = r.id
  AND r.is_system = 0
  AND r.role_key LIKE 'custom_%';
