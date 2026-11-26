-- Aethravox Studios — Migración 027: Permisos atómicos por rango (capabilities)
--
-- Extiende el sistema de rangos (migración 026) con permisos atómicos por
-- ACCIÓN (no por sección). El Owner marca qué acciones puede ejecutar cada
-- rango: banear, suspender, entrar al panel, ver auditoría, gestionar webs,
-- responder tickets, etc. Un usuario con un rango puede hacer todo lo que su
-- rol del sistema le permita (baseline) MÁS lo que su rango le conceda.
-- El Owner del sistema tiene bypass total (ignora rangos y permisos).
--
-- Tabla:
--   rank_perms(rank_id, perm)  — capabilities atómicas concedidas a un rango.
--   perm es un string corto tipo "users.ban.temporary", "admin.access", etc.
--   El catálogo completo vive en src/models.rs (PERMISSIONS).

CREATE TABLE IF NOT EXISTS rank_perms (
    rank_id INTEGER NOT NULL,
    perm    TEXT    NOT NULL,
    UNIQUE(rank_id, perm)
);
