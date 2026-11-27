use aethravox_core::{ArisChatDto, ArisMessageDto, ArisMode};
use egui::Rounding;

use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    ui.horizontal(|ui| {
        // Left chat list
        card_frame(p).show(ui, |ui| {
            ui.set_width(230.0);
            ui.set_min_height(ui.available_height());
            ui.horizontal(|ui| {
                ui.heading("✨ Aris");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ Nuevo").clicked() {
                        let next_id = app.aris_chats.len() as i64 + 1;
                        app.aris_chats.insert(
                            0,
                            ArisChatDto {
                                id: next_id,
                                title: format!("Chat #{next_id}"),
                                pinned: false,
                                updated_at: "Ahora".into(),
                            },
                        );
                        app.active_aris_chat_id = next_id;
                    }
                });
            });
            ui.separator();

            for c in &app.aris_chats {
                let active = c.id == app.active_aris_chat_id;
                if ui.selectable_label(active, &c.title).clicked() {
                    app.active_aris_chat_id = c.id;
                }
            }
        });

        ui.add_space(8.0);

        // Right Aris conversation
        card_frame(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(ui.available_height());

            ui.horizontal(|ui| {
                ui.heading("Aris Core");
                badge_pill(
                    ui,
                    "En línea",
                    p.success.gamma_multiply(0.18),
                    p.success,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for mode in [ArisMode::Pensando, ArisMode::Rapido, ArisMode::Instantaneo] {
                        if ui
                            .selectable_label(app.aris_mode == mode, mode.label())
                            .clicked()
                        {
                            app.aris_mode = mode;
                        }
                    }
                });
            });
            ui.separator();

            let msg_h = (ui.available_height() - 56.0).max(160.0);
            egui::ScrollArea::vertical()
                .id_salt("aris_msgs")
                .max_height(msg_h)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for m in &app.aris_messages {
                        let is_user = m.role == "user";
                        egui::Frame::none()
                            .fill(if is_user {
                                p.accent_secondary.gamma_multiply(0.14)
                            } else {
                                p.bg_elevated
                            })
                            .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(if is_user { "Tú" } else { "✨ Aris" })
                                            .strong()
                                            .color(if is_user {
                                                p.accent_secondary
                                            } else {
                                                p.accent_primary
                                            }),
                                    );
                                    if let Some(model) = &m.model {
                                        ui.label(
                                            egui::RichText::new(model).small().color(p.text_muted),
                                        );
                                    }
                                });
                                ui.label(&m.content);
                            });
                        ui.add_space(6.0);
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let enter = ui
                    .add(
                        egui::TextEdit::singleline(&mut app.aris_draft)
                            .desired_width((ui.available_width() - 95.0).max(120.0))
                            .hint_text("Pregúntale a Aris…"),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if primary_button(ui, p, "Enviar").clicked() || enter {
                    let prompt = app.aris_draft.trim().to_string();
                    if !prompt.is_empty() {
                        let next_id = app.aris_messages.len() as i64 + 1;
                        app.aris_messages.push(ArisMessageDto {
                            id: next_id,
                            role: "user".into(),
                            content: prompt.clone(),
                            model: None,
                            created_at: "Ahora".into(),
                        });
                        app.aris_messages.push(ArisMessageDto {
                            id: next_id + 1,
                            role: "assistant".into(),
                            content: format!(
                                "[Modo {}] Recibido: \"{}\". Conecta el servidor aethravox-server para streaming SSE real con Ollama/Gemini.",
                                app.aris_mode.label(),
                                prompt
                            ),
                            model: Some("aris-desktop".into()),
                            created_at: "Ahora".into(),
                        });
                        app.aris_draft.clear();
                    }
                }
            });
        });
    });
}
