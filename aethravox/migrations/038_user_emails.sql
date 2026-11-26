-- Sección 6 — correos internos verificados + solicitudes de cambio.
--
-- NOTA: el reset de contraseña (6.4) REUSA la tabla `recovery_tokens` que
-- ya existe en 001_init.sql (token, user_id, expires_at, used). No creamos
-- `password_reset_tokens` aparte para no duplicar schema.

-- Correos internos vinculados a un usuario del hub. Un usuario puede tener
-- varios; uno es el primario. `verified_at` es NULL hasta que el usuario
-- valida el código de 8 dígitos que le llegó por AethraMail.
CREATE TABLE IF NOT EXISTS user_emails (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    email       TEXT    NOT NULL,
    verified_at TEXT,
    is_primary  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, email),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_user_emails_user ON user_emails(user_id);
CREATE INDEX IF NOT EXISTS idx_user_emails_verified ON user_emails(user_id, verified_at);

-- Solicitudes de verificación/cambio de correo (6.2). El usuario pide
-- vincular `new_email`; el hub genera `code` (8 dígitos), lo manda por
-- AethraMail, y el usuario lo introduce. Al consumirse, se inserta en
-- user_emails (verified) y se marca used_at.
CREATE TABLE IF NOT EXISTS email_change_requests (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    new_email   TEXT    NOT NULL,
    code        TEXT    NOT NULL,
    expires_at  TEXT    NOT NULL,
    used_at     TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_email_change_user ON email_change_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_email_change_code ON email_change_requests(user_id, code);
