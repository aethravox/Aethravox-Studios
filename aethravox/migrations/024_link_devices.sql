-- Aethravox Studios — Migración 024: Vinculación de dispositivos (estilo WhatsApp Web)
--
-- Objetivo: permitir tener varios dispositivos activos con la misma cuenta SIN
-- volver a meter la contraseña. El dispositivo "principal" (el que inició
-- sesión con user+pass) genera un código QR / numérico de un uso; el dispositivo
-- "vinculado" lo escanea o teclea y recibe una sesión de menor privilegio que
-- se auto-cierra a los 3 días de inactividad. El principal puede revocarla.
--
-- Si un segundo dispositivo intenta LOGUEARSE (con user+pass) en una cuenta que
-- ya tiene una sesión primary abierta, esa sesión primary se cierra — para
-- obligar a usar vinculación en vez de re-login.

-- ===== Columnas nuevas en sessions =====
-- device_name: etiqueta legible ("Chrome · Windows", "Safari · iPhone", …).
--              NULL para sesiones creadas antes de esta migración.
-- is_primary: 1 = sesión iniciada con contraseña (la "principal", solo una
--              activa a la vez por cuenta). 0 = sesión vinculada (puede haber
--              varias). Las sesiones existentes se asumen primary (= 1).
-- linked_by_user_id: para sesiones vinculadas, el user_id del principal que la
--              aprobó (debería coincidir con user_id; se guarda aparte por
--              auditoría). NULL para primary.
-- link_expires_at: para vinculadas, fecha de auto-cierre (created + 3 días,
--                  rolling: se extiende +3 días en cada actividad). NULL para
--                  primary (usan expires_at normal).
ALTER TABLE sessions ADD COLUMN device_name TEXT;
ALTER TABLE sessions ADD COLUMN is_primary INTEGER NOT NULL DEFAULT 1;
ALTER TABLE sessions ADD COLUMN linked_by_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE sessions ADD COLUMN link_expires_at DATETIME;

-- ===== Tabla de códigos de vinculación =====
-- Cada vez que el principal abre /vincular, se genera una fila nueva:
--   code         → token aleatorio URL-safe de 16 chars (va dentro del QR como
--                  parte de la URL https://sitio/vincular?c=<code>).
--   numeric_code → 6 dígitos para teclear a mano en el dispositivo vinculado.
--   user_id      → el principal que lo generó (autoridad que aprueba).
--   expires_at   → now + 60 s (rotación rápida: si no se usa, caduca solo).
--   used_at      → momento en que se consumió (NULL = libre).
--   used_by_token→ token de la sesión vinculada creada a partir (auditoría).
CREATE TABLE IF NOT EXISTS link_codes (
    code            TEXT PRIMARY KEY,
    numeric_code    TEXT NOT NULL,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at      DATETIME NOT NULL,
    used_at         DATETIME,
    used_by_token   TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_link_codes_user        ON link_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_link_codes_numeric      ON link_codes(numeric_code);
CREATE INDEX IF NOT EXISTS idx_link_codes_expires      ON link_codes(expires_at);
