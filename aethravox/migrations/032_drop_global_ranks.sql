-- Aethravox Studios — Migración 032: Eliminar el sistema de rangos globales.
--
-- Los rangos globales (tablas `ranks`, `rank_perms`, `rank_web_perms` y la
-- columna `users.rank_id`) eran un bundle ADITIVO de permisos extra encima
-- del rol del sistema. Con el sistema de roles configurables (migraciones
-- 029/030/031) los roles ya cubren todo lo que un rango podía conceder, así
-- que el bundle de rangos queda obsoleto y se elimina.
--
-- Esta migración:
--   1) Dropea las 3 tablas de rangos globales (ranks, rank_perms, rank_web_perms).
--   2) Elimina la columna `users.rank_id` con ALTER TABLE DROP COLUMN.
--
-- Los `conversation_ranks` (rangos por grupo de chat) NO se tocan: son una
-- característica distinta de permisos por grupo, no del sistema global.

-- 1) Tablas de rangos globales. Estas tablas no tienen FKs declaradas
--    (sus columnas *_id son INTEGER planas, sin REFERENCES), así que
--    dropearlas es seguro sin tocar datos de otras tablas.
DROP TABLE IF EXISTS rank_web_perms;
DROP TABLE IF EXISTS rank_perms;
DROP TABLE IF EXISTS ranks;

-- 2) Eliminar la columna rank_id de users.
--
--    Usamos ALTER TABLE DROP COLUMN (SQLite >= 3.35.0). El binario de este
--    proyecto enlaza libsqlite3-sys 0.30 (SQLite 3.45+), que lo soporta.
--
--    NO usamos el patrón "tabla temporal + copia + DROP TABLE users + RENAME"
--    porque `users(id)` es padre de varias tablas con FOREIGN KEY ...
--    ON DELETE RESTRICT (bans.banned_by, suspensions.suspended_by,
--    moderator_logs.moderator_id). Con PRAGMA foreign_keys = ON (que sqlx
--    habilita), DROP TABLE users hace un DELETE implícito de todas las
--    filas, y esas hijas RESTRICT lo bloquean → "FOREIGN KEY constraint
--    failed". ALTER DROP COLUMN no toca las filas y evita el problema.
--
--    rank_id no participa en ningún índice ni restricción UNIQUE/PK (verif.
--    en migraciones 026 y posteriores), así que el DROP COLUMN es seguro.
ALTER TABLE users DROP COLUMN rank_id;
