-- Aethravox Studios — Migración 036: Rol Invitado (guest stateless) + cuenta
-- sistema para tickets de invitado.
--
-- Sección 3.1 — Rol "Invitado":
--   * posición mínima (0, por debajo de Usuario=100) → jerarquía más baja.
--   * is_system=1 (no se puede borrar desde el admin), is_protected=0.
--   * permisos atómicos de solo lectura (docs.view). El gating fuerte por
--     módulo (chat/friends/dashboard/transmitir/admin/notifications) lo hace
--     el routing (require_auth sobre las hard-protected), no los permisos.
--   * visibilidad de webs públicas (role_web_perms nueva + web_role_perms
--     legacy, que es la que lee list_webs_for_role en el hub).
--
-- Sección 3.5 — Tickets de invitado:
--   * tickets.user_id es NOT NULL REFERENCES users(id). Un invitado stateless
--     no tiene user_id. Resolvemos con UNA cuenta sistema sembrada "invitado"
--     (role='usuario' para pasar el CHECK de users.role; role_id=invitado) que
--     actúa de dueño de los tickets fileados por invitados sin sesión.
--   * Columna tickets.guest_ip (ALTER seguro) para traceability + rate-limit.
--     NULL en tickets de usuarios logueados; la IP real en tickets de invitados.
--
-- El invitado del middleware (auth_mw::resolve_user_or_guest) es un User
-- sintético EN MEMORIA (id=0, sin fila en users) — no se crea fila por visita,
-- tal como pide la spec 3.2. Esta cuenta "invitado" es una sola fila sembrada,
-- independiente, solo dueño de tickets anónimos.
--
-- Nota: no se usa PRAGMA foreign_keys ni rebuild de tablas. sqlx envuelve cada
-- migración en su propia transacción (ver comentario en 031), por lo que un
-- PRAGMA fuera de transacción no tendría efecto y un rebuild rompería.

-- ============================================================
-- 1) Rol Invitado.
-- ============================================================
INSERT INTO roles (name, role_key, color, position, description, is_system, is_protected)
VALUES ('Invitado', 'invitado', '#6b7280', 0,
        'Invitado sin cuenta. Solo lectura en lo público.', 1, 0);

-- ============================================================
-- 2) Permisos atómicos del invitado: solo lectura mínima.
--    El acceso a módulos bloqueados lo decide el routing, no estos permisos.
-- ============================================================
INSERT INTO role_perms (role_id, perm)
SELECT r.id, p.perm FROM (
    SELECT 'docs.view' AS perm
) p
CROSS JOIN roles r
WHERE r.role_key = 'invitado';

-- ============================================================
-- 3) Visibilidad de webs — role_web_perms (tabla nueva).
--    El invitado ve las webs públicas del hub: docs, communities, support.
--    (hub/login/dashboard están ocultos del listado vía HIDDEN_SLUGS.)
-- ============================================================
INSERT INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id
FROM roles r
CROSS JOIN webs w
WHERE r.role_key = 'invitado'
  AND w.slug IN ('docs', 'communities', 'support');

-- ============================================================
-- 4) Visibilidad de webs — web_role_perms (tabla legacy): NO se siembra.
--    web_role_perms.role tiene CHECK (role IN ('owner','admin','staff',
--    'usuario')), así que INSERT de 'invitado' la rechaza. La visibilidad del
--    invitado vive SOLO en role_web_perms (sección 3), y list_webs_for_role
--    la lee por role_key cuando role='invitado' (JOIN roles → role_web_perms,
--    ver db/webs.rs). No hace falta tocar web_role_perms para el invitado.
-- ============================================================

-- ============================================================
-- 5) Cuenta sistema "invitado" — dueño de tickets anónimos.
--    role='usuario' (pasa el CHECK de users.role); role_id=invitado (perms).
--    password_hash='' → no se puede loguear (verify de hash vacío falla).
--    INSERT OR IGNORE por idempotencia si se re-corre la migración.
-- ============================================================
INSERT OR IGNORE INTO users (username, email, password_hash, role, role_id)
SELECT 'invitado', 'invitado@aethravox.local', '', 'usuario', r.id
FROM roles r
WHERE r.role_key = 'invitado';

-- ============================================================
-- 6) Traceability de tickets de invitado: IP del invitado que fileó.
--    NULL → ticket de usuario logueado; IP real → ticket de invitado.
--    También sirve de marcador para distinguir tickets invitado de los
--    'bug' normales fileados por usuarios con sesión.
-- ============================================================
ALTER TABLE tickets ADD COLUMN guest_ip TEXT;
