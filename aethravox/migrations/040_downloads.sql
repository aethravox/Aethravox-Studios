-- 040_downloads.sql — Sección 7: página /descargas (clientes nativos).
--
-- Tabla de ejecutables subidos desde el panel admin (Windows / Linux / Android
-- activos; macOS / iOS se muestran como "Próximamente" sin archivos). Los
-- binarios se guardan en data/downloads/ y se sirven vía
-- nest_service("/descargas/files", ServeDir::new("data/downloads")) en main.rs.
--
-- Estructura: cada fila = un archivo. Se agrupan por (platform, app_slug) en la
-- página pública: Platform → App (nombrada) → archivos. Una app puede tener
-- varios archivos (versiones / variantes: setup, portable, etc.).
CREATE TABLE IF NOT EXISTS downloads (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    platform     TEXT    NOT NULL,               -- windows|linux|android|macos|ios
    app_slug     TEXT    NOT NULL,               -- agrupa archivos de una misma app
    app_name     TEXT    NOT NULL,               -- nombre a mostrar de la app
    version      TEXT    NOT NULL DEFAULT '',    -- ej: "1.0.0"
    filename     TEXT    NOT NULL,               -- nombre guardado en data/downloads/
    orig_name    TEXT    NOT NULL DEFAULT '',    -- nombre original (display al descargar)
    size_bytes   INTEGER NOT NULL DEFAULT 0,
    checksum     TEXT,                           -- sha256 (opcional; NULL por ahora)
    notes        TEXT,                           -- notas de release (opcional)
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_downloads_platform ON downloads(platform);
CREATE INDEX IF NOT EXISTS idx_downloads_app      ON downloads(app_slug);
