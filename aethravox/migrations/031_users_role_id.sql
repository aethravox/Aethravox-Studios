-- Migración 031: migrar users.role (TEXT) → users.role_id (FK a roles).
--
-- Pasos:
--  1) Añadir users.role_id INTEGER (nullable durante la migración).
--  2) Para cada user, buscar el rol cuyo role_key coincide con su
--     users.role actual (owner/admin/staff/usuario) y setear role_id.
--  3) Mantener users.role como columna legada (deprecada) por si hay
--     código que la lea; se vacía gradualmente. NO se borra para no romper
--     selects existentes en el binario actual. Se eliminará en la migración
--     040 una vez que todo el código pase a usar role_id.
--
-- NOTA: no se usa BEGIN/COMMIT explícito porque sqlx ya aplica cada
-- migración dentro de su propia transacción. Anidar daría
-- "cannot start a transaction within a transaction".

-- 1) Añadir columna role_id (FK a roles; ON DELETE RESTRICT impide borrar
--    un rol que aún tiene usuarios asignados — el admin debe reasignar
--    primero, lo cual el handler de delete_role ya fuerza).
ALTER TABLE users ADD COLUMN role_id INTEGER REFERENCES roles(id) ON DELETE RESTRICT;

-- 2) Poblar role_id según el role (TEXT) actual de cada user.
--    Como el CHECK de users.role limitaba a los 4 valores del sistema,
--    todos coinciden con un role_key existente (creados en la migración 030).
UPDATE users
SET role_id = (SELECT id FROM roles WHERE role_key = users.role)
WHERE role_id IS NULL;

-- 3) La columna users.role (TEXT) se mantiene por retrocompat. El código
--    nuevo debe leer users.role_id y JOIN con roles para obtener role_key,
--    position, name, etc.

-- ============================================================
-- Índices auxiliares
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_users_role_id ON users(role_id);
