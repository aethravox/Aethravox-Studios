-- Aethravox Studios — Migración 015: Estado de mensajes (check marks estilo WhatsApp)
-- Permite tracking: sent (✓) → delivered (✓✓ gris) → read (✓✓ rojo)
-- Solo aplica a DMs (en grupos no se tracking por miembro).

ALTER TABLE messages ADD COLUMN status TEXT NOT NULL DEFAULT 'sent';
