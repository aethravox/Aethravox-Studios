-- Aethravox Studios — Migración 022: Ciclo de vida de tickets (asistencia)
--
-- El admin puede ahora: reclamar, cerrar, rechazar o terminar un ticket.
-- Añadimos dos columnas a `tickets`:
--   reclamado_por  INTEGER  -> id del admin que reclama el ticket (NULL = nadie)
--   resolucion     TEXT     -> 'terminado' | 'rechazado' | NULL (solo importa
--                              cuando status='closed')
--
-- No tocamos el CHECK de `status` (seguimos usando open/answered/closed): la
-- semántica fina (reclamado / terminado / rechazado) la aporta la combinación
-- de status + reclamado_por + resolucion. ALTER TABLE ADD COLUMN es seguro en
-- SQLite: no reconstruye la tabla, no pierde datos y no necesita transacción
-- ni PRAGMA foreign_keys (evitamos el patrón de rebuild que sí los necesita y
-- que sqlx podría envolver en transacción, rompiendo el PRAGMA).

ALTER TABLE tickets ADD COLUMN reclamado_por INTEGER;
ALTER TABLE tickets ADD COLUMN resolucion TEXT;

CREATE INDEX IF NOT EXISTS idx_tickets_reclamado ON tickets(reclamado_por);
