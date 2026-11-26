-- Aethravox Studios — Migración 037: Sistema de categorías del hub (Sección 4).
--
-- Rediseña el hub de una lista plana de webs a tiras horizontales por
-- categoría (scroll-snap, CSS puro). Cada "app" del hub (web) puede pertenecer
-- a una categoría. El hub muestra hasta 18 apps por categoría + botón "Ver más"
-- que lleva a /categoria/:slug (todas las apps de esa categoría).
--
-- Modelo:
--   categories (id, name, slug UNIQUE, position, created_at)
--   app_categories (app_id → webs.id, category_id → categories.id)  many-to-many
--
-- "Transmitir" era una card HARDCODEADA en hub.html. Acá se siembra como web
-- (slug='transmitir') para que entre al sistema de categorías como una app más.
-- La card hardcodeada se elimina del template del hub (rediseño Sección 4.3).

-- ============================================================
-- 1) Tabla categories.
-- ============================================================
CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    -- slug URL-safe único: /categoria/:slug. P.ej. 'comunicacion'.
    slug        TEXT NOT NULL UNIQUE,
    -- Orden de aparición en el hub (menor = más arriba).
    position    INTEGER NOT NULL DEFAULT 100,
    created_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_position ON categories(position);

-- ============================================================
-- 2) Tabla app_categories (many-to-many webs ↔ categories).
--    Una app puede estar en una categoría (por ahora 1:N simple; la tabla
--    soporta N:M por si una app entra en varias en el futuro).
-- ============================================================
CREATE TABLE IF NOT EXISTS app_categories (
    app_id       INTEGER NOT NULL,
    category_id  INTEGER NOT NULL,
    UNIQUE(app_id, category_id),
    FOREIGN KEY (app_id)      REFERENCES webs(id)       ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_app_categories_cat ON app_categories(category_id);

-- ============================================================
-- 3) Sembrar "Transmitir" como web (antes hardcoded en hub.html).
--    is_public=0 (no es pública anónima); la ruta /transmitir está tras
--    require_auth (hard-protected). web_role_perms para owner/admin/staff/
--    usuario (NO invitado: el invitado no ve Transmitir).
-- ============================================================
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public)
VALUES ('transmitir', 'Transmitir',
        'Comparte tu pantalla en cualquier dispositivo — sin cables, sin USB.',
        '📡', '/transmitir', 0);

INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT w.id, r.role FROM webs w
CROSS JOIN (SELECT 'owner' AS role UNION ALL SELECT 'admin' UNION ALL SELECT 'staff' UNION ALL SELECT 'usuario') r
WHERE w.slug = 'transmitir';

-- ============================================================
-- 4) Sembrar categorías base.
-- ============================================================
INSERT INTO categories (name, slug, position) VALUES
    ('Comunicación', 'comunicacion', 10),
    ('Creatividad',  'creatividad',  20),
    ('Herramientas', 'herramientas', 30),
    ('Asistencia',   'asistencia',   40);

-- ============================================================
-- 5) Asignar apps existentes a categorías.
--    Comunicación: chat, friends, communities
--    Creatividad:  docs, aextube, playground
--    Herramientas: transmitir
--    Asistencia:   support
-- ============================================================
INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id FROM webs w JOIN categories c ON
       (w.slug = 'chat'        AND c.slug = 'comunicacion')
    OR (w.slug = 'friends'     AND c.slug = 'comunicacion')
    OR (w.slug = 'communities' AND c.slug = 'comunicacion')
    OR (w.slug = 'docs'        AND c.slug = 'creatividad')
    OR (w.slug = 'aextube'     AND c.slug = 'creatividad')
    OR (w.slug = 'playground'  AND c.slug = 'creatividad')
    OR (w.slug = 'transmitir'  AND c.slug = 'herramientas')
    OR (w.slug = 'support'     AND c.slug = 'asistencia');
