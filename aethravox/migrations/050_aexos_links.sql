-- Aethravox Studios — Migración 050: Vínculos de cuentas reales de Aethravox Studios dentro de aexOS
-- Solo se guarda el vínculo (owner_user_id -> linked_user_id / nombre de usuario),
-- nunca la contraseña del usuario vinculado.

CREATE TABLE IF NOT EXISTS aexos_links (
    owner_user_id  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    linked_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at     DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (owner_user_id, linked_user_id)
);
CREATE INDEX IF NOT EXISTS idx_aexos_links_owner ON aexos_links(owner_user_id);
