-- Aethravox Studios — Migración 018: Marcar secciones custom existentes como is_custom=1
-- Las secciones creadas antes del fix de create_web (V3.13) tenían is_custom=0
-- incluso si fueron creadas por el owner. Esta migración las corrige.
-- Solo marca como custom las que NO son del sistema (login, hub, chat, etc.).

UPDATE webs SET is_custom = 1
WHERE slug NOT IN (
    'login', 'register', 'recover', 'reset',
    'hub', 'dashboard', 'admin',
    'chat', 'friends', 'playground', 'docs'
)
AND is_custom = 0;
