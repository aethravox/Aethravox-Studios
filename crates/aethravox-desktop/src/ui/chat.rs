use aethravox_core::ChatMessageDto;
use egui::Rounding;

use crate::app::AethravoxApp;
use crate::theme::{card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    ui.columns(2, |cols| {
        // Note: egui::Ui::columns splits 50/50, so let's keep a clean custom split using SidePanel-like layout or horizontal strip
        let _ = cols;
    });

    ui.horizontal(|ui| {
        // Left conversations list
        card_frame(p).show(ui, |ui| {
            ui.set_width(240.0);
            ui.set_min_height(ui.available_height());
            ui.heading("Chats");
            ui.separator();
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
                            1.0,
                            if active { p.accent_primary } else { p.border_subtle },
                        ))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0));

                    let resp = frame
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new(&c.title).strong());
                            ui.label(
                                egui::RichText::new(&c.last_message)
                                    .small()
                                    .color(p.text_muted),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        app.active_conv_id = c.id;
                    }
                    ui.add_space(6.0);
                }
            });
        });

        ui.add_space(8.0);

        // Right message pane
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
                            .stroke(egui::Stroke::new(1.0, p.border_subtle))
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
                let enter_pressed = ui
                    .add(
                        egui::TextEdit::singleline(&mut app.chat_draft)
                            .desired_width((ui.available_width() - 95.0).max(120.0))
                            .hint_text("Escribe un mensaje…"),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if primary_button(ui, p, "Enviar").clicked() || enter_pressed {
                    let txt = app.chat_draft.trim().to_string();
                    if !txt.is_empty() {
                        let next_id = app.chat_messages.len() as i64 + 1;
                        app.chat_messages.push(ChatMessageDto {
                            id: next_id,
                            conversation_id: app.active_conv_id,
                            sender: app
                                .hub
                                .as_ref()
                                .map(|h| h.user.display.clone())
                                .unwrap_or_else(|| "Tú".into()),
                            content: txt,
                            created_at: "Ahora".into(),
                            is_mine: true,
                        });
                        app.chat_draft.clear();
                    }
                }
            });
        });
    });

    let _ = elevated_frame;
}
