use aethravox_core::CreateGroupRequest;
use egui::Rounding;

use crate::api::{spawn_create_group, spawn_start_dm};
use crate::app::AethravoxApp;
use crate::theme::{card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut clicked_conv: Option<i64> = None;
    let mut send_requested = false;
    let mut start_dm: Option<String> = None;
    let mut create_group: Option<CreateGroupRequest> = None;

    let mobile = app.is_mobile_layout(&ctx);

    if mobile {
        if !app.mobile_detail_open {
            show_conv_list(
                app,
                ui,
                p,
                ui.available_width(),
                &mut clicked_conv,
                &mut start_dm,
                &mut create_group,
            );
        } else {
            show_chat_pane(app, ui, p, true, &mut send_requested);
        }
    } else {
        ui.horizontal(|ui| {
            show_conv_list(
                app,
                ui,
                p,
                250.0,
                &mut clicked_conv,
                &mut start_dm,
                &mut create_group,
            );
            ui.add_space(8.0);
            show_chat_pane(app, ui, p, false, &mut send_requested);
        });
    }

    if let Some(id) = clicked_conv {
        app.mobile_detail_open = true;
        app.chat_open_conversation(&ctx, id);
    }
    if send_requested {
        app.chat_send_draft(&ctx);
    }
    if let Some(username) = start_dm {
        if !app.offline_mode && app.token.is_some() {
            spawn_start_dm(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                username,
            );
        }
    }
    if let Some(req) = create_group {
        if !app.offline_mode && app.token.is_some() {
            spawn_create_group(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
}

fn show_conv_list(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    width: f32,
    clicked_conv: &mut Option<i64>,
    start_dm: &mut Option<String>,
    create_group: &mut Option<CreateGroupRequest>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(width);
        ui.set_min_height(ui.available_height());
        ui.horizontal(|ui| {
            ui.heading("Chats");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let toggle_lbl = if app.chat_new_panel { "✕" } else { "+ Nuevo" };
                if ui.button(toggle_lbl).clicked() {
                    app.chat_new_panel = !app.chat_new_panel;
                }
            });
        });

        if app.chat_new_panel {
            elevated_frame(p).show(ui, |ui| {
                ui.label(egui::RichText::new("Nuevo DM").strong().small());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.chat_dm_username)
                            .desired_width((ui.available_width() - 75.0).max(100.0))
                            .hint_text("@usuario"),
                    );
                    if ui.button("Abrir").clicked() && !app.chat_dm_username.trim().is_empty() {
                        *start_dm = Some(app.chat_dm_username.trim().to_string());
                        app.chat_dm_username.clear();
                        app.chat_new_panel = false;
                    }
                });
                ui.separator();
                ui.label(egui::RichText::new("Nuevo Grupo").strong().small());
                ui.add(
                    egui::TextEdit::singleline(&mut app.chat_group_name)
                        .hint_text("Nombre del grupo"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.chat_group_desc)
                        .hint_text("Descripción (opcional)"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.chat_group_members)
                        .hint_text("Miembros: user1, user2"),
                );
                if primary_button(ui, p, "Crear grupo").clicked()
                    && !app.chat_group_name.trim().is_empty()
                {
                    let members = app
                        .chat_group_members
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    *create_group = Some(CreateGroupRequest {
                        name: app.chat_group_name.trim().to_string(),
                        description: Some(app.chat_group_desc.trim().to_string()),
                        members,
                    });
                    app.chat_group_name.clear();
                    app.chat_group_desc.clear();
                    app.chat_group_members.clear();
                    app.chat_new_panel = false;
                }
            });
            ui.add_space(6.0);
        }

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
                    *clicked_conv = Some(c.id);
                }
                ui.add_space(6.0);
            }
        });
    });
}

fn show_chat_pane(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    show_back: bool,
    send_requested: &mut bool,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());

        let active_title = app
            .conversations
            .iter()
            .find(|c| c.id == app.active_conv_id)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Conversación".to_string());

        ui.horizontal_wrapped(|ui| {
            if show_back && ui.button("← Chats").clicked() {
                app.mobile_detail_open = false;
            }
            ui.heading(&active_title);
        });
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
                *send_requested = true;
                resp.request_focus();
            }
        });
    });
}
