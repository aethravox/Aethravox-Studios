-- Migración 033: sistema de títulos.
--
-- Un título es un badge público que el Owner puede otorgar a un usuario
-- (p.ej. "Creador", "Streamer", "Dueño de X por colaboración"). A diferencia
-- del rol (que controla permisos), el título es COSMÉTICO: solo se muestra.
--
-- Reglas de visualización (definidas por el propio usuario desde /perfil):
--   - El usuario puede tener TODOS los títulos que le den.
--   - En su perfil público elige hasta 3 para mostrar.
--   - De esos 3 elige 1 como "principal": ese se ve junto a su nombre en
--     todo el sitio (chat, amigos, admin, etc.) sin abrir el perfil.
--
-- Quién puede crear/editar/borrar títulos y quién puede asignarlos:
--   - Permiso nuevo `titles.manage`. Por defecto SOLO el Owner lo tiene
--     (ver migración 030b_roles_revoke_roles_manage.sql que retira
--     roles.manage del Admin y esta migración no se lo concede).
--
-- Modelo:
--   titles           — catálogo de títulos (name, color, effect, description).
--   user_titles      — qué títulos tiene cada usuario (asignaciones).
--   users.primary_title_id        — título principal (FK, nullable).
--   users.visible_title_1..3      — los 3 que se muestran en el perfil.

CREATE TABLE IF NOT EXISTS titles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    -- Color del badge en formato composite (igual que roles.color):
    -- "#hex|text|opacity" o "#hex~#hex2|text|opacity". Lo parsea
    -- role_color::parse_role_color al servir /title-colors.css.
    color       TEXT,
    -- Efecto visual opcional: 'none' | 'glow' | 'shimmer' | 'gradient'.
    -- El CSS correspondiente lo genera /title-colors.css.
    effect      TEXT    NOT NULL DEFAULT 'none'
                    CHECK (effect IN ('none','glow','shimmer','gradient')),
    description TEXT,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Asignaciones: qué usuario tiene qué título. Un usuario puede tener N.
CREATE TABLE IF NOT EXISTS user_titles (
    user_id     INTEGER NOT NULL,
    title_id    INTEGER NOT NULL,
    granted_by  INTEGER,
    granted_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, title_id),
    FOREIGN KEY (user_id)  REFERENCES users(id)    ON DELETE CASCADE,
    FOREIGN KEY (title_id) REFERENCES titles(id)   ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id)  ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_user_titles_user  ON user_titles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_titles_title ON user_titles(title_id);

-- Título principal del usuario (se ve junto al nombre en todo el sitio).
-- Nullable: si no tiene título asignado o no eligió principal, no se muestra.
ALTER TABLE users ADD COLUMN primary_title_id INTEGER REFERENCES titles(id) ON DELETE SET NULL;

-- Los 3 títulos visibles en el perfil público (slots 1..3). Nullable cada uno.
-- Solo se permiten títulos que el usuario tenga en user_titles; el handler
-- /perfil/select valida eso antes de guardar.
ALTER TABLE users ADD COLUMN visible_title_1 INTEGER REFERENCES titles(id) ON DELETE SET NULL;
ALTER TABLE users ADD COLUMN visible_title_2 INTEGER REFERENCES titles(id) ON DELETE SET NULL;
ALTER TABLE users ADD COLUMN visible_title_3 INTEGER REFERENCES titles(id) ON DELETE SET NULL;
