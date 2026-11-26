-- Aethravox Studios — Migración 048: Créditos diarios, modos, pensamiento profundo y multimodal en Aris + URL AethraMail

CREATE TABLE IF NOT EXISTS aris_daily_credits (
    user_id         INTEGER PRIMARY KEY,
    daily_used      INTEGER NOT NULL DEFAULT 0,
    last_reset_date DATE NOT NULL DEFAULT (date('now')),
    updated_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS aris_credit_logs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id             INTEGER NOT NULL,
    chat_id             INTEGER,
    mode                TEXT NOT NULL DEFAULT 'instantaneo',
    thinking_enabled    INTEGER NOT NULL DEFAULT 0,
    thinking_level      TEXT NOT NULL DEFAULT 'muy_bajo',
    attachments_summary TEXT NOT NULL DEFAULT '',
    credits_cost        INTEGER NOT NULL DEFAULT 1,
    created_at          DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_aris_credit_logs_user ON aris_credit_logs(user_id, id DESC);

ALTER TABLE aris_messages ADD COLUMN meta_json TEXT;

INSERT OR IGNORE INTO aris_config (key, value) VALUES
    ('daily_credits_limit', '120'),
    ('aethramail_url', '');
