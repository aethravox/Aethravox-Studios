-- Aethravox Studios — Migración 044: Proveedores ilimitados, rotación avanzada, límites diarios y bitácora
--
-- 1) Elimina la restricción UNIQUE sobre api_providers.provider_type para permitir
--    múltiples instancias del mismo tipo (ej: varios OpenAI-compat, varias cuentas Groq, etc.).
-- 2) Añade default_model, is_preset y priority a api_providers y siembra presets adicionales.
-- 3) Añade daily_cap (tope preventivo diario opcional) a api_keys preservando todas las keys existentes.
-- 4) Crea api_rotation_logs para auditar peticiones, fallbacks y health checks en tiempo real.
-- 5) Siembra opciones de rotación en aris_config.

-- ============================================================
-- 1) Respaldar api_keys antes de reconstruir api_providers
--    (foreign_keys = ON está activo en SQLite)
-- ============================================================
CREATE TABLE _api_keys_backup AS SELECT * FROM api_keys;
DROP TABLE api_keys;

-- ============================================================
-- 2) Reconstruir api_providers sin UNIQUE(provider_type)
-- ============================================================
CREATE TABLE _api_providers_new (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    base_url      TEXT NOT NULL,
    default_model TEXT NOT NULL DEFAULT 'default',
    is_preset     INTEGER NOT NULL DEFAULT 0,
    priority      INTEGER NOT NULL DEFAULT 10,
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO _api_providers_new (
    id, name, provider_type, base_url, default_model, is_preset, priority, is_active, created_at
)
SELECT
    id,
    name,
    provider_type,
    base_url,
    CASE provider_type
        WHEN 'gemini' THEN 'gemini-2.5-flash'
        WHEN 'groq' THEN 'llama-3.1-8b-instant'
        WHEN 'anthropic' THEN 'claude-3-5-sonnet-latest'
        ELSE 'google/gemma-2-9b-it:free'
    END,
    1,
    id,
    is_active,
    created_at
FROM api_providers;

DROP TABLE api_providers;
ALTER TABLE _api_providers_new RENAME TO api_providers;

-- Sembrar proveedores predeterminados adicionales si no existen
INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
SELECT 'OpenAI', 'openai_compat', 'https://api.openai.com/v1', 'gpt-4o-mini', 1, 4, 1
WHERE NOT EXISTS (SELECT 1 FROM api_providers WHERE lower(name) = 'openai');

INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
SELECT 'Anthropic', 'anthropic', 'https://api.anthropic.com/v1', 'claude-3-5-sonnet-latest', 1, 5, 1
WHERE NOT EXISTS (SELECT 1 FROM api_providers WHERE lower(name) = 'anthropic');

INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
SELECT 'Together AI', 'openai_compat', 'https://api.together.xyz/v1', 'meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo', 1, 6, 1
WHERE NOT EXISTS (SELECT 1 FROM api_providers WHERE lower(name) = 'together ai');

INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
SELECT 'Mistral', 'openai_compat', 'https://api.mistral.ai/v1', 'mistral-small-latest', 1, 7, 1
WHERE NOT EXISTS (SELECT 1 FROM api_providers WHERE lower(name) = 'mistral');

INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
SELECT 'DeepSeek', 'openai_compat', 'https://api.deepseek.com/v1', 'deepseek-chat', 1, 8, 1
WHERE NOT EXISTS (SELECT 1 FROM api_providers WHERE lower(name) = 'deepseek');

-- ============================================================
-- 3) Reconstruir api_keys con daily_cap y restaurar datos
-- ============================================================
CREATE TABLE api_keys (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id         INTEGER NOT NULL,
    api_key             TEXT NOT NULL,
    label               TEXT NOT NULL DEFAULT '',
    is_active           INTEGER NOT NULL DEFAULT 1,
    total_uses          INTEGER NOT NULL DEFAULT 0,
    daily_uses          INTEGER NOT NULL DEFAULT 0,
    daily_cap           INTEGER,
    daily_start         DATE NOT NULL DEFAULT (date('now')),
    last_used_at        DATETIME,
    rate_limited_until  DATETIME,
    last_error          TEXT,
    created_at          DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (provider_id) REFERENCES api_providers(id) ON DELETE CASCADE
);

INSERT INTO api_keys (
    id, provider_id, api_key, label, is_active,
    total_uses, daily_uses, daily_cap, daily_start,
    last_used_at, rate_limited_until, last_error, created_at
)
SELECT
    id, provider_id, api_key, label, is_active,
    total_uses, daily_uses, NULL, daily_start,
    last_used_at, rate_limited_until, last_error, created_at
FROM _api_keys_backup;

DROP TABLE _api_keys_backup;
CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider_id, is_active);

-- ============================================================
-- 4) Bitácora persistente de rotación, fallbacks y health checks
-- ============================================================
CREATE TABLE IF NOT EXISTS api_rotation_logs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    source        TEXT NOT NULL DEFAULT 'aris_chat',
    channel_label TEXT NOT NULL DEFAULT 'Aris Chat',
    provider_name TEXT NOT NULL,
    key_label     TEXT NOT NULL DEFAULT '',
    model         TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL,
    latency_ms    INTEGER NOT NULL DEFAULT 0,
    detail        TEXT NOT NULL DEFAULT '',
    created_at    DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_api_rotation_logs_id ON api_rotation_logs(id DESC);

-- ============================================================
-- 5) Parámetros de rotación automática en aris_config
-- ============================================================
INSERT OR IGNORE INTO aris_config (key, value) VALUES
    ('rotation_enabled', '1'),
    ('rotation_strategy', 'round_robin'),
    ('cooldown_minutes', '60'),
    ('auto_failover', '1');
