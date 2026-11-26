-- Aethravox Studios — Migración 020: Panel de asistencia/sugerencias (Fase 6)
-- Sistema de tickets: los usuarios envían sugerencias, reportes o consultas.
-- El admin los ve y responde. Thread de mensajes por ticket.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tickets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL,
    ticket_type     TEXT NOT NULL DEFAULT 'sugerencia'
                    CHECK (ticket_type IN ('sugerencia', 'reporte', 'consulta', 'bug')),
    subject         TEXT NOT NULL,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'answered', 'closed')),
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    answered_at     DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ticket_messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id       INTEGER NOT NULL,
    from_user_id    INTEGER NOT NULL,
    is_admin        INTEGER NOT NULL DEFAULT 0,
    body            TEXT NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (ticket_id) REFERENCES tickets(id) ON DELETE CASCADE,
    FOREIGN KEY (from_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tickets_user ON tickets(user_id);
CREATE INDEX IF NOT EXISTS idx_tickets_status ON tickets(status);
CREATE INDEX IF NOT EXISTS idx_ticket_msgs_ticket ON ticket_messages(ticket_id);

-- Registrar la web "support" en el hub
INSERT OR IGNORE INTO webs (slug, name, description, icon, url, is_public) VALUES
    ('support', 'Asistencia', 'Envía sugerencias, reportes o consultas al equipo.', '🛟', '/support', 0);

INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'owner'   FROM webs WHERE slug = 'support';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'admin'   FROM webs WHERE slug = 'support';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'staff'   FROM webs WHERE slug = 'support';
INSERT OR IGNORE INTO web_role_perms (web_id, role) SELECT id, 'usuario' FROM webs WHERE slug = 'support';

-- Añadir "Tickets" al panel admin
-- (no es una web del hub, es una ruta admin: /admin/tickets)
