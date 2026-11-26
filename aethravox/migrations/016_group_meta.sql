-- Aethravox Studios — Migración 016: Meta-datos de grupos (Fase 2)
-- Añade descripción, URL de imagen y owner_id a las conversaciones.
-- Solo aplica a grupos (type='group'). Los DMs ignoran estos campos.

ALTER TABLE conversations ADD COLUMN description TEXT;
ALTER TABLE conversations ADD COLUMN image_url TEXT;
-- owner_id: quien creó el grupo. Se usa para permisos (transferir ownership, etc.)
-- NULL para DMs. Igual a created_by por ahora, pero separado para permitir transferencia.
ALTER TABLE conversations ADD COLUMN owner_id INTEGER REFERENCES users(id) ON DELETE SET NULL;

-- Para grupos existentes, setear owner_id = created_by
UPDATE conversations SET owner_id = created_by WHERE type = 'group' AND owner_id IS NULL;
