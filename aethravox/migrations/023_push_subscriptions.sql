-- Suscripciones Push (Web Push API) por usuario.
-- Cada vez que un navegador pide suscribirse a push, guardamos aquí la
-- subscription info (endpoint + claves p256dh/auth) para poder enviarle
-- notificaciones incluso cuando NO tiene la pestaña del chat abierta
-- (el backend detecta "sin WS activo" y dispara un web-push).
--
-- Un usuario puede tener varias suscripciones (un device por navegador/teléfono).
-- Si una suscripción expira o el usuario la revoca, la borramos.
CREATE TABLE IF NOT EXISTS push_subscriptions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    endpoint    TEXT    NOT NULL,          -- URL del push service (FCM/Mozilla/APNS)
    p256dh      TEXT    NOT NULL,          -- clave pública ECDH del cliente (base64url)
    auth        TEXT    NOT NULL,          -- clave auth (base64url)
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    UNIQUE(endpoint)                       -- un endpoint = una fila (misma suscripción no se duplica)
);
CREATE INDEX IF NOT EXISTS idx_push_subs_user ON push_subscriptions(user_id);
