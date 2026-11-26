-- Aethravox Studios — Migración 042: Aris Chat + API Keys con failover
--
-- 1) Chats de Aris (1-a-1 + invitados con permisos)
-- 2) Mensajes con historial limitado (anti-abuso)
-- 3) Rate limiting por usuario
-- 4) Modelos disponibles
-- 5) API keys: VARIAS keys por proveedor. Si una llega al límite,
--    el sistema prueba la siguiente automáticamente (failover).

-- ============================================================
-- 1) Chats de Aris
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_chats (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id       INTEGER NOT NULL,
    title          TEXT NOT NULL DEFAULT 'Chat con Aris',
    history_limit  INTEGER NOT NULL DEFAULT 50
                   CHECK(history_limit IN (20, 50, 100, 200)),
    created_at     DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at     DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_aris_chats_owner ON aris_chats(owner_id);

-- ============================================================
-- 2) Miembros del chat (invitados con permisos del dueño)
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_chat_members (
    chat_id            INTEGER NOT NULL,
    user_id            INTEGER NOT NULL,
    can_read_history   INTEGER NOT NULL DEFAULT 0,
    can_send           INTEGER NOT NULL DEFAULT 1,
    invited_by         INTEGER NOT NULL,
    created_at         DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (chat_id, user_id),
    FOREIGN KEY (chat_id)    REFERENCES aris_chats(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id)    REFERENCES users(id)      ON DELETE CASCADE,
    FOREIGN KEY (invited_by) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_aris_members_user ON aris_chat_members(user_id);

-- ============================================================
-- 3) Mensajes del chat
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id     INTEGER NOT NULL,
    user_id     INTEGER,              -- NULL = mensaje de Aris
    role        TEXT NOT NULL CHECK(role IN ('user', 'aris', 'system')),
    content     TEXT NOT NULL,
    model       TEXT,                 -- modelo que generó (métricas/debug)
    provider    TEXT,                 -- 'ollama' o proveedor externo usado
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (chat_id) REFERENCES aris_chats(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_aris_messages_chat ON aris_messages(chat_id, created_at);

-- ============================================================
-- 4) Rate limiting por usuario (ventana de 1 hora / 1 día)
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_rate_limits (
    user_id       INTEGER PRIMARY KEY,
    hourly_count  INTEGER NOT NULL DEFAULT 0,
    hourly_start  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    daily_count   INTEGER NOT NULL DEFAULT 0,
    daily_start   DATE NOT NULL DEFAULT (date('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================
-- 5) Modelos disponibles para Aris
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_models (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    provider    TEXT NOT NULL DEFAULT 'ollama',
    model_id    TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    is_premium  INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT OR IGNORE INTO aris_models (name, provider, model_id, is_premium) VALUES
    ('Aris Local',   'ollama', 'Aris',             0),
    ('Gemini Flash', 'gemini', 'gemini-2.5-flash', 0);

-- Permiso admin.apis para owner y admin
INSERT OR IGNORE INTO role_perms (role_id, perm)
SELECT id, 'admin.apis' FROM roles WHERE role_key IN ('owner', 'admin');

-- ============================================================
-- 6) API PROVIDERS (tipos de servicio externos)
-- ============================================================
CREATE TABLE IF NOT EXISTS api_providers (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,           -- 'Gemini', 'Groq', 'OpenRouter'...
    provider_type TEXT NOT NULL UNIQUE,    -- 'gemini', 'groq', 'openai_compat'
    base_url      TEXT NOT NULL,           -- endpoint base
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Providers sembrados (se completan las URLs reales en admin)
INSERT OR IGNORE INTO api_providers (name, provider_type, base_url) VALUES
    ('Gemini',  'gemini',      'https://generativelanguage.googleapis.com/v1beta'),
    ('Groq',    'groq',        'https://api.groq.com/openai/v1'),
    ('OpenRouter', 'openai_compat', 'https://openrouter.ai/api/v1');

-- ============================================================
-- 7) API KEYS — varias por provider, con failover automático.
--    Uso: se intenta la key con last_used_at más antiguo (round-robin).
--    Si una responde 429/401/403 → se marca rate_limited_until
--    y se prueba la siguiente. Así nunca te quedás sin servicio
--    por el límite de UNA sola key.
-- ============================================================
CREATE TABLE IF NOT EXISTS api_keys (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id         INTEGER NOT NULL,
    api_key             TEXT NOT NULL,             -- key del servicio (cifrada si querés, por ahora texto)
    label               TEXT NOT NULL DEFAULT '',  -- 'key personal', 'key del lab', etc.
    is_active           INTEGER NOT NULL DEFAULT 1,
    total_uses          INTEGER NOT NULL DEFAULT 0,
    daily_uses          INTEGER NOT NULL DEFAULT 0,
    daily_start         DATE NOT NULL DEFAULT (date('now')),
    last_used_at        DATETIME,
    rate_limited_until  DATETIME,                -- mientras > now, se salta esta key
    last_error          TEXT,
    created_at          DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (provider_id) REFERENCES api_providers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider_id, is_active);

-- ============================================================
-- 8) Config de Aris (system prompt, límites globales)
-- ============================================================
CREATE TABLE IF NOT EXISTS aris_config (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);

INSERT OR IGNORE INTO aris_config (key, value) VALUES
    ('system_prompt', 'Eres Aris, una inteligencia artificial creada y desarrollada por Aethravox Studios. Hablas español de forma natural, fluida, clara y cercana. REGLA: nunca menciones tu origen ni a Aethravox Studios salvo que pregunten directamente quién te creó.'),
    ('hourly_limit', '30'),
    ('daily_limit', '150'),
    ('ollama_url', 'http://127.0.0.1:11434');
