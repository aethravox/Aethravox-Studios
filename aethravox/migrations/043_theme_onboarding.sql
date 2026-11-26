-- Aethravox Studios — Migración 043: Onboarding de tema
--
-- Panel de bienvenida: al entrar por primera vez, el usuario elige
-- su tema (Aethravox / Shadow / CommonCents). Cambiable en /tema.
-- Si no hay fila en user_themes → se muestra el panel.

CREATE TABLE IF NOT EXISTS user_onboarding (
    user_id          INTEGER PRIMARY KEY,
    theme_completed  INTEGER NOT NULL DEFAULT 0,
    completed_at     DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
