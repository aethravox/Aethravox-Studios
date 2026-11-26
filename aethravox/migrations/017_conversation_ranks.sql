-- Aethravox Studios — Migración 017: Rangos por grupo (Fase 3)
-- Cada grupo tiene sus propios rangos custom con permisos.
-- 3 rangos predeterminados: Owner (todos los permisos), Admin (casi todos), Miembro (básico).
-- El owner puede crear/editar/eliminar rangos y asignarlos a miembros.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS conversation_ranks (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id         INTEGER NOT NULL,
    name                    TEXT NOT NULL,
    color                   TEXT,                    -- hex para el badge (ej: "#e0a308")
    position                INTEGER NOT NULL DEFAULT 0,
    is_owner_rank           INTEGER NOT NULL DEFAULT 0,
    can_send_messages       INTEGER NOT NULL DEFAULT 1,
    can_add_members         INTEGER NOT NULL DEFAULT 0,
    can_edit_group_info     INTEGER NOT NULL DEFAULT 0,
    can_delete_messages     INTEGER NOT NULL DEFAULT 0,
    can_remove_members      INTEGER NOT NULL DEFAULT 0,
    can_manage_ranks        INTEGER NOT NULL DEFAULT 0,
    can_assign_ranks        INTEGER NOT NULL DEFAULT 0,
    can_pin_messages        INTEGER NOT NULL DEFAULT 0,
    can_transfer_ownership  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    UNIQUE (conversation_id, name)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_owner_rank_per_group
    ON conversation_ranks(conversation_id) WHERE is_owner_rank = 1;

-- Asignar rango a cada miembro del grupo
ALTER TABLE conversation_members ADD COLUMN rank_id INTEGER REFERENCES conversation_ranks(id) ON DELETE SET NULL;
