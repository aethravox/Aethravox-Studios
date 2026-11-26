-- Migración 029: sistema de roles configurables + permisos granulares por rol.
--
-- Antes: users.role era un TEXT con 4 valores hardcoded (owner/admin/staff/
-- usuario) y el baseline de permisos vivía en código (baseline_has_perm).
-- Las "capabilities" extra vivían en `ranks` + `rank_perms`.
--
-- Ahora: roles configurables desde el admin. Cada rol tiene:
--   (a) capabilities atómicas (role_perms) — ban, ban-ip, entrar a /admin,
--       ver cada sección, borrar usuarios, etc. CADA COSA es un permiso.
--   (b) visibilidad de secciones (role_web_perms) — qué webs del hub ve.
--   (c) nivel jerárquico (position) — para can_act_on (no actuar sobre
--       alguien de nivel superior).
--   (d) flag is_system — los 4 roles base son del sistema y no se pueden
--       borrar (sí editar). El owner tiene is_protected=1 (no se puede
--       cambiar su nombre ni quitarle permisos).
--
-- Rangos (`ranks` + `rank_perms`) SIGUEN existiendo como bundle ADITIVO
-- extra encima del rol. La diferencia: un usuario puede tener 1 rol (obligatorio)
-- + 1 rango (opcional, para extras puntuales).
--
-- El motor de autorización (Db::user_has_perm) queda:
--   if user.role.is_owner { return true; }    -- bypass
--   if role_has_perm(user.role_id, perm)     -- baseline del rol
--       || rank_has_perm(user.rank_id, perm) -- extras del rango
--       return true;
--   false

CREATE TABLE IF NOT EXISTS roles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    -- "key" internamente: owner / admin / staff / usuario / custom_<id>.
    -- El owner se identifica por role_key = 'owner' (no por name, que se
    -- puede renombrar). role_key es UNIQUE y nunca cambia tras crear.
    role_key    TEXT    NOT NULL UNIQUE,
    color       TEXT,
    -- Posición jerárquica. Owner = 1000, Admin = 500, Staff = 200,
    -- Usuario = 100. Roles custom eligen la suya (0-999).
    -- can_act_on(target) → target.position < my.position (estricto).
    position    INTEGER NOT NULL DEFAULT 100,
    -- Descripción corta mostrada en el admin ("Rol base para usuarios
    -- comunes", "Moderador con permisos de chat", etc.).
    description TEXT,
    -- Roles del sistema: no se pueden borrar. El Owner además es
    -- is_protected: no se puede cambiar su name/role_key/position ni
    -- quitarle permisos (siempre los tiene todos vía bypass).
    is_system     INTEGER NOT NULL DEFAULT 0,
    is_protected  INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (is_system IN (0, 1)),
    CHECK (is_protected IN (0, 1))
);

-- Permisos atómicos por rol. CADA acción del sistema es un permiso aquí.
-- Catálogo canónico en models.rs::PERMISSIONS. Esta tabla solo guarda
-- qué claves del catálogo están concedidas a cada rol.
CREATE TABLE IF NOT EXISTS role_perms (
    role_id INTEGER NOT NULL,
    perm    TEXT    NOT NULL,
    UNIQUE(role_id, perm),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

-- Visibilidad de secciones del hub por rol. Reemplaza a web_role_perms
-- (que se mantiene por compat, pero la fuente de verdad pasa a ser esta).
-- Una sección es visible por un rol si existe la fila.
CREATE TABLE IF NOT EXISTS role_web_perms (
    role_id INTEGER NOT NULL,
    web_id  INTEGER NOT NULL,
    UNIQUE(role_id, web_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (web_id)  REFERENCES webs(id)  ON DELETE CASCADE
);

-- Índice para el lookup "¿el rol X tiene el permiso Y?" — hot path.
CREATE INDEX IF NOT EXISTS idx_role_perms_perm ON role_perms(perm);
