# Aethravox Studios (Desktop Rebuild — `egui`)

Rama de reconstrucción enfocada en cliente nativo de escritorio en Rust (`eframe` / `egui`), desacoplando progresivamente el backend del renderizado web (Askama/HTML/CSS).

## Estructura del Workspace

- `crates/aethravox-core`: DTOs compartidos, modelos de temas (`Aethravox`, `Shadow`, `CommonCents`) y contratos JSON.
- `crates/aethravox-desktop`: Cliente nativo de escritorio en `eframe` + `egui` con sistema de temas propio, Login/Registro/Preview, Hub por categorías, Chat, Aris IA, Neon Canvas 2D nativo y Ajustes.
- `aethravox`: Servidor backend Axum + SQLite con los nuevos endpoints `/api/v1/*` (autenticación Bearer + JSON) mientras se migran y retiran las vistas Askama/HTML.
- `aethramail`: Servicio independiente de correo interno (`mTLS`).

## Cómo ejecutar en tu PC

1. Arrancar el servidor backend:
   ```bash
   cargo run -p aethravox
   ```
2. En otra terminal, arrancar el cliente nativo de escritorio:
   ```bash
   cargo run -p aethravox-desktop
   ```
