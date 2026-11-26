-- Aethravox Studios — Migración 047: AethraMail como servicio web independiente
-- AethraMail es un servicio externo / web aparte de correo y recuperación de cuentas.
-- Se retira de app_categories para que no figure como app interna dentro de las tiras del hub.

DELETE FROM app_categories
WHERE app_id IN (SELECT id FROM webs WHERE slug = 'correo');
