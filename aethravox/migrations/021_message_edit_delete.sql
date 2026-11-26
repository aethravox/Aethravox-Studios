-- Aethravox Studios — Migración 021: Edición y borrado de mensajes de chat
-- Permite edición (con timestamp) y borrado lógico (soft delete) de mensajes.
-- Reglas:
--   - Solo el autor puede editar/borrar SUS mensajes.
--   - Solo dentro de los 30 minutos posteriores al envío (cliente + server check).
--   - deleted_at != NULL → el mensaje se renderiza como "Mensaje eliminado"
--     (el body se reemplaza en el backend para no exponer contenido).
--   - edited_at != NULL → el mensaje muestra "editado" junto a la hora.

ALTER TABLE messages ADD COLUMN edited_at  TEXT;  -- ISO8601 timestamp o NULL
ALTER TABLE messages ADD COLUMN deleted_at TEXT;  -- ISO8601 timestamp o NULL

-- Índice por si en el futuro queremos queries de "todos los editados en X conv"
CREATE INDEX IF NOT EXISTS idx_messages_edited  ON messages (edited_at)  WHERE edited_at  IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_messages_deleted ON messages (deleted_at) WHERE deleted_at IS NOT NULL;
