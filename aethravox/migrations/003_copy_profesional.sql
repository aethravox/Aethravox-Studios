-- Aethravox Studios — Migración 003: copy profesional
-- Reemplaza los textos RPG de las 3 webs base por textos profesionales.
-- Solo actualiza si las filas existen; no toca nada más.

PRAGMA foreign_keys = ON;

UPDATE webs SET
    name = 'Panel principal',
    description = 'Acceso a todas las secciones del estudio.'
WHERE slug = 'hub';

UPDATE webs SET
    name = 'Mi perfil',
    description = 'Datos de cuenta, avatar, sesiones activas.'
WHERE slug = 'dashboard';

UPDATE webs SET
    name = 'Acceso',
    description = 'Iniciar sesión o crear cuenta.'
WHERE slug = 'login';

-- El footer de los templates ya no dice "Gremio Digital" (cambio en base.html).
-- El "Esperando aventureros..." del log se cambió en main.rs.
-- El tagline de la página de login se cambió en login.html a "Acceso al Estudio".
