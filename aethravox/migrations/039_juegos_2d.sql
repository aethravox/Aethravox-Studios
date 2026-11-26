-- Aethravox Studios — Migración 039: Juegos 2D (Neon Canvas Ultimate)
--
-- Nueva categoría "Juegos 2D" (position 50, debajo de Asistencia) + primera
-- app de la categoría: Neon Canvas, un juego canvas 2D de dibujo neón con
-- modos libre, desafío, zen, VS IA y diario, powerups, logros y multijugador
-- WiFi. Toda la lógica del juego es client-side (/static/js/juegos +
-- /static/css/juegos); la ruta /juegos/neon-canvas está tras require_auth
-- (protected_routes, igual que /playground).
--
-- Visibilidad: owner/admin/staff/usuario (NO invitado). is_public=0.

-- 1) Categoría "Juegos 2D".
INSERT OR IGNORE INTO categories (name, slug, position)
VALUES ('Juegos 2D', 'juegos-2d', 50);

-- 2) App "Neon Canvas" como web del hub.
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public)
VALUES ('neon-canvas', 'Neon Canvas',
        'Juego de dibujo neón: modos libre, desafío, zen, VS IA y diario. Powerups, logros y multijugador WiFi.',
        '🎮', '/juegos/neon-canvas', 0);

-- 3) Permisos por rol (owner/admin/staff/usuario — NO invitado).
INSERT OR IGNORE INTO web_role_perms (web_id, role)
SELECT w.id, r.role FROM webs w
CROSS JOIN (SELECT 'owner' AS role UNION ALL SELECT 'admin' UNION ALL SELECT 'staff' UNION ALL SELECT 'usuario') r
WHERE w.slug = 'neon-canvas';

-- 4) Asignar la app a la categoría "Juegos 2D".
INSERT OR IGNORE INTO app_categories (app_id, category_id)
SELECT w.id, c.id FROM webs w JOIN categories c ON
       (w.slug = 'neon-canvas' AND c.slug = 'juegos-2d');
