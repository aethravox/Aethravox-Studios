-- AethraMail — migración inicial.
-- DB propia, separada de la del hub.
--
-- Dominios internos (@shadow.sac, @aethra.sac, @cents.sac) se controlan en
-- app config (AETHRAMAIL_DOMAINS), no en DB: así se añaden sin migrar.

-- Cuentas de correo. La dirección completa es `address || '@' || domain`.
-- Login propio (5.6): address + domain + contraseña (independiente del hub).
CREATE TABLE IF NOT EXISTS email_accounts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    address       TEXT    NOT NULL,
    domain        TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(address, domain)
);

CREATE INDEX IF NOT EXISTS idx_email_accounts_lookup
    ON email_accounts(address, domain);

-- Correos. from_address / to_address guardan la dirección completa
-- (ej. "felipe@shadow.sac"). folder = inbox | sent | drafts | trash.
CREATE TABLE IF NOT EXISTS emails (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    from_address TEXT    NOT NULL,
    to_address   TEXT    NOT NULL,
    subject      TEXT    NOT NULL DEFAULT '',
    body         TEXT    NOT NULL DEFAULT '',
    folder       TEXT    NOT NULL DEFAULT 'inbox',
    read_at      TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_emails_to_folder
    ON emails(to_address, folder);

CREATE INDEX IF NOT EXISTS idx_emails_from_folder
    ON emails(from_address, folder);
