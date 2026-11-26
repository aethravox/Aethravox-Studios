-- Aethravox Studios — Migración 041: Sistema de temas + nuevas categorías
--
-- 1) Tabla user_themes: cada usuario elige su tema visual.
-- 2) Seed de 3 temas (Aethravox, Shadow Studios, CommonCents).
-- 3) Nuevas categorías del hub (reemplaza las de prueba).
-- 4) Aris se siembra como app en categoría IA.

-- ============================================================
-- 1) Temas de usuario
-- ============================================================
CREATE TABLE IF NOT EXISTS user_themes (
    user_id     INTEGER PRIMARY KEY,
    theme       TEXT NOT NULL DEFAULT 'aethravox'
                CHECK(theme IN ('aethravox', 'shadow', 'commoncents')),
    updated_at  DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ============================================================
-- 2) Seed de categorías profesionales (reemplaza las de prueba)
-- ============================================================
-- Limpiar categorías de prueba y sus asignaciones
DELETE FROM app_categories;
DELETE FROM categories;

-- Insertar nuevas categorías
INSERT INTO categories (name, slug, position) VALUES
    ('Inteligencia Artificial', 'ia', 10),
    ('Comunicación', 'comunicacion', 20),
    ('Herramientas', 'herramientas', 30),
    ('Creatividad', 'creatividad', 40),
    ('Entretenimiento', 'entretenimiento', 50),
    ('Productividad', 'productividad', 60),
    ('Asistencia', 'asistencia', 70),
    ('Personal', 'personal', 80);

-- ============================================================
-- 3) Reasignar apps existentes a nuevas categorías
-- ============================================================
INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id FROM webs w JOIN categories c ON
       (w.slug = 'chat'        AND c.slug = 'comunicacion')
    OR (w.slug = 'friends'     AND c.slug = 'comunicacion')
    OR (w.slug = 'communities' AND c.slug = 'comunicacion')
    OR (w.slug = 'transmitir'  AND c.slug = 'herramientas')
    OR (w.slug = 'docs'        AND c.slug = 'creatividad')
    OR (w.slug = 'aextube'     AND c.slug = 'creatividad')
    OR (w.slug = 'playground'  AND c.slug = 'creatividad')
    OR (w.slug = 'neon-canvas' AND c.slug = 'entretenimiento')
    OR (w.slug = 'aexos'       AND c.slug = 'herramientas')
    OR (w.slug = 'support'     AND c.slug = 'asistencia');

-- ============================================================
-- 4) Aris: nueva app de IA
-- ============================================================
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public)
VALUES ('aris', 'Aris',
        'Tu asistente de inteligencia artificial. Conversa, pregunta, crea.',
        '🤖', '/aris', 0);

-- Permisos: owner/admin/staff/usuario (no invitado)
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT w.id, r.role FROM webs w
CROSS JOIN (SELECT 'owner' AS role UNION ALL SELECT 'admin' UNION ALL SELECT 'staff' UNION ALL SELECT 'usuario') r
WHERE w.slug = 'aris';

INSERT OR IGNORE INTO role_web_perms (role_id, web_id)
SELECT r.id, w.id FROM roles r
JOIN webs w ON w.slug = 'aris'
WHERE r.role_key IN ('owner', 'admin', 'staff', 'usuario');

-- Aris en categoría IA
INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id FROM webs w JOIN categories c
WHERE w.slug = 'aris' AND c.slug = 'ia';

-- ============================================================
-- 5) Descargas y Correo en sus categorías
-- ============================================================
INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id FROM webs w JOIN categories c ON
       (w.slug = 'descargas' AND c.slug = 'herramientas')
    OR (w.slug = 'correo'    AND c.slug = 'productividad');
