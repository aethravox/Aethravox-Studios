use aethravox_core::PlaygroundGenRequest;

use crate::api::spawn_playground_generate;
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut generate = false;

    card_frame(p).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("🧪 Playground");
            badge_pill(
                ui,
                "HTML · CSS · JS",
                p.accent_secondary.gamma_multiply(0.16),
                p.accent_secondary,
            );
            if ui.button("Plantilla Neón").clicked() {
                app.pg_html = "<div class=\"card\">\n  <h1>Aethravox Lab</h1>\n  <p>Prototipo rápido en vivo.</p>\n  <button id=\"btn\">Pulsar</button>\n</div>".into();
                app.pg_css = "body { background:#06030c; color:#f8f8fc; font-family:sans-serif; display:grid; place-items:center; min-height:100vh; }\n.card { padding:24px 32px; border:1px solid #e0a308; border-radius:14px; background:#0d0818; text-align:center; }\nbutton { background:#e0a308; color:#0c0a12; border:none; padding:10px 18px; border-radius:8px; font-weight:bold; cursor:pointer; }".into();
                app.pg_js = "document.getElementById('btn').onclick = () => alert('¡Hola desde Playground!');".into();
            }
            if ui.button("📋 Copiar HTML completo").clicked() {
                let full_doc = format!(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{}</style></head><body>{}<script>{}</script></body></html>",
                    app.pg_css, app.pg_html, app.pg_js
                );
                ctx.copy_text(full_doc);
                app.status_banner = Some("HTML completo copiado al portapapeles.".into());
            }
            if primary_button(ui, p, "▶ Abrir en navegador").clicked() {
                let full_doc = format!(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{}</style></head><body>{}<script>{}</script></body></html>",
                    app.pg_css, app.pg_html, app.pg_js
                );
                let encoded = encode_data_url(&full_doc);
                ctx.open_url(egui::OpenUrl::new_tab(encoded));
            }
        });

        ui.add_space(8.0);

        // Barra copiloto Aris
        elevated_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("✨ Aris Copilot:").strong().color(p.accent_primary));
                let enter = ui
                    .add(
                        egui::TextEdit::singleline(&mut app.pg_prompt)
                            .desired_width((ui.available_width() - 150.0).max(120.0))
                            .hint_text("Describe qué quieres generar en HTML/CSS/JS…"),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let btn_txt = if app.pg_busy {
                    "Generando…"
                } else {
                    "Generar con Aris"
                };
                if (primary_button(ui, p, btn_txt).clicked() || enter) && !app.pg_busy {
                    generate = true;
                }
            });
            if !app.pg_note.is_empty() {
                ui.label(egui::RichText::new(&app.pg_note).small().color(p.success));
            }
            if let Some(err) = &app.pg_error {
                ui.label(egui::RichText::new(err).small().color(p.danger));
            }
        });

        ui.add_space(10.0);

        // Tabs HTML / CSS / JS
        ui.horizontal(|ui| {
            for (idx, label) in [(0usize, "HTML"), (1, "CSS"), (2, "JavaScript")] {
                if ui.selectable_label(app.pg_tab == idx, label).clicked() {
                    app.pg_tab = idx;
                }
            }
        });
        ui.separator();

        let code_h = (ui.available_height() - 16.0).max(220.0);
        egui::ScrollArea::vertical()
            .id_salt("pg_editor")
            .max_height(code_h)
            .show(ui, |ui| {
                let target = match app.pg_tab {
                    0 => &mut app.pg_html,
                    1 => &mut app.pg_css,
                    _ => &mut app.pg_js,
                };
                ui.add(
                    egui::TextEdit::multiline(target)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(22),
                );
            });
    });

    if generate && !app.pg_prompt.trim().is_empty() {
        let prompt = app.pg_prompt.trim().to_string();
        if app.offline_mode || app.token.is_none() {
            app.pg_html = format!(
                "<div class=\"demo\">\n  <h2>{}</h2>\n  <p>Generado en modo local. Conecta el servidor para usar Aris Playground.</p>\n</div>",
                prompt
            );
            app.pg_note = "Plantilla generada localmente.".into();
        } else {
            app.pg_busy = true;
            app.pg_error = None;
            spawn_playground_generate(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                PlaygroundGenRequest {
                    prompt,
                    current_html: app.pg_html.clone(),
                },
            );
        }
    }
}

fn encode_data_url(html: &str) -> String {
    let mut out = String::from("data:text/html;charset=utf-8,");
    for b in html.as_bytes() {
        let safe = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if safe {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}
