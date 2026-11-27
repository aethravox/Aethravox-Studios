use egui::Rounding;

use crate::app::AethravoxApp;
use crate::theme::{card_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut clicked_conv: Option<i64> = None;
    let mut send_requested = false;

    ui.horizontal(|ui| {
        // ---------- Lista de conversaciones ----------
        card_frame(p).show(ui, |ui| {
            ui.set_width(240.0);
            ui.set_min_height(ui.available_height());
            ui.heading("Chats");
            ui.separator();

            if app.conversations.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No tienes conversaciones todavía.")
                        .small()
                        .color(p.text_muted),
                );
            }

            egui::ScrollArea::vertical().id_salt("chat_convs").show(ui, |ui| {
                for c in &app.conversations {
                    let active = c.id == app.active_conv_id;
                    let frame = egui::Frame::none()
                        .fill(if active {
                            p.accent_primary.gamma_multiply(0.16)
                        } else {
                            p.bg_elevated
                        })
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if active { p.accent_primary } else { p.border_subtle },
                        ))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0));

                    let resp = frame
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&c.title).strong());
                                if c.unread_count > 0 {
                                    ui.label(
                                        egui::RichText::new(format!("● {}", c.unread_count))
                                            .small()
                                            .color(p.accent_primary),
                                    );
                                }
                            });
                            ui.label(
                                egui::RichText::new(&c.last_message)
                                    .small()
                                    .color(p.text_muted),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        clicked_conv = Some(c.id);
                    }
                    ui.add_space(6.0);
                }
            });
        });

        ui.add_space(8.0);

        // ---------- Panel de mensajes ----------
        card_frame(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(ui.available_height());

            let active_title = app
                .conversations
                .iter()
                .find(|c| c.id == app.active_conv_id)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| "Conversación".to_string());

            ui.heading(&active_title);
            ui.separator();

            if let Some(err) = &app.chat_error {
                ui.label(egui::RichText::new(err).small().color(p.danger));
            }

            let msg_height = (ui.available_height() - 56.0).max(160.0);
            egui::ScrollArea::vertical()
                .id_salt("chat_msgs")
                .max_height(msg_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for m in &app.chat_messages {
                        if m.conversation_id != app.active_conv_id {
                            continue;
                        }
                        let bg = if m.is_mine {
                            p.accent_primary.gamma_multiply(0.16)
                        } else {
                            p.bg_elevated
                        };
                        egui::Frame::none()
                            .fill(bg)
                            .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                            .rounding(Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&m.sender).strong().small());
                                    ui.label(
                                        egui::RichText::new(&m.created_at)
                                            .small()
                                            .color(p.text_muted),
                                    );
                                });
                                ui.label(&m.content);
                            });
                        ui.add_space(6.0);
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.chat_draft)
                        .desired_width((ui.available_width() - 95.0).max(120.0))
                        .hint_text("Escribe un mensaje…"),
                );
                let enter_pressed =
                    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                let label = if app.chat_sending { "Enviando…" } else { "Enviar" };
                let clicked = primary_button(ui, p, label).clicked();

                if clicked || enter_pressed {
                    send_requested = true;
                    resp.request_focus();
                }
            });
        });
    });

    if let Some(id) = clicked_conv {
        app.chat_open_conversation(&ctx, id);
    }
    if send_requested {
        app.chat_send_draft(&ctx);
    }
}
