use aethravox_core::{ArisChatDto, ArisMessageDto, ArisMode};
use egui::Rounding;

use crate::api::{
    spawn_aris_chats, spawn_aris_create_chat, spawn_aris_credits, spawn_aris_delete_chat,
    spawn_aris_messages, spawn_aris_pin_chat, spawn_aris_send,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut create_chat = false;
    let mut select_chat: Option<i64> = None;
    let mut pin_chat: Option<i64> = None;
    let mut delete_chat: Option<i64> = None;
    let mut send_prompt: Option<String> = None;
    let mut refresh = false;

    let mobile = app.is_mobile_layout(&ctx);

    if mobile {
        if !app.mobile_detail_open {
            show_aris_list(
                app,
                ui,
                p,
                ui.available_width(),
                &mut create_chat,
                &mut refresh,
                &mut select_chat,
                &mut pin_chat,
                &mut delete_chat,
            );
        } else {
            show_aris_pane(app, ui, p, true, &mut send_prompt);
        }
    } else {
        ui.horizontal(|ui| {
            show_aris_list(
                app,
                ui,
                p,
                250.0,
                &mut create_chat,
                &mut refresh,
                &mut select_chat,
                &mut pin_chat,
                &mut delete_chat,
            );
            ui.add_space(8.0);
            show_aris_pane(app, ui, p, false, &mut send_prompt);
        });
    }

    if refresh && !app.offline_mode && app.token.is_some() {
        spawn_aris_chats(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
        spawn_aris_credits(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }

    if create_chat {
        app.mobile_detail_open = true;
        if app.offline_mode || app.token.is_none() {
            let next_id = app.aris_chats.iter().map(|c| c.id).max().unwrap_or(0) + 1;
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
            app.aris_messages.clear();
        } else {
            spawn_aris_create_chat(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                None,
            );
        }
    }

    if let Some(cid) = select_chat {
        app.active_aris_chat_id = cid;
        app.aris_error = None;
        app.mobile_detail_open = true;
        if !app.offline_mode && app.token.is_some() {
            spawn_aris_messages(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cid,
            );
        }
    }

    if let Some(cid) = pin_chat {
        if app.offline_mode || app.token.is_none() {
            if let Some(c) = app.aris_chats.iter_mut().find(|c| c.id == cid) {
                c.pinned = !c.pinned;
            }
        } else {
            spawn_aris_pin_chat(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cid,
            );
        }
    }

    if let Some(cid) = delete_chat {
        if app.offline_mode || app.token.is_none() {
            app.aris_chats.retain(|c| c.id != cid);
            if app.active_aris_chat_id == cid {
                app.active_aris_chat_id = app.aris_chats.first().map(|c| c.id).unwrap_or(0);
            }
        } else {
            spawn_aris_delete_chat(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cid,
            );
        }
    }

    if let Some(prompt) = send_prompt {
        if app.offline_mode || app.token.is_none() {
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
                    "[Modo {}] Recibido: \"{}\". Inicia sesión conectado al servidor para respuestas reales de Aris (Ollama / Failover).",
                    app.aris_mode.label(),
                    prompt
                ),
                model: Some("aris-demo".into()),
                created_at: "Ahora".into(),
            });
        } else if app.active_aris_chat_id > 0 {
            app.aris_busy = true;
            app.aris_error = None;
            let next_id = app.aris_messages.len() as i64 + 1000;
            app.aris_messages.push(ArisMessageDto {
                id: next_id,
                role: "user".into(),
                content: prompt.clone(),
                model: None,
                created_at: "Ahora".into(),
            });
            spawn_aris_send(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                app.active_aris_chat_id,
                prompt,
                app.aris_mode,
            );
        } else {
            app.aris_error = Some("Crea o selecciona un chat de Aris primero.".into());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn show_aris_list(
    app: &AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    width: f32,
    create_chat: &mut bool,
    refresh: &mut bool,
    select_chat: &mut Option<i64>,
    pin_chat: &mut Option<i64>,
    delete_chat: &mut Option<i64>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(width);
        ui.set_min_height(ui.available_height());
        ui.horizontal(|ui| {
            ui.heading("✨ Aris");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Nuevo").clicked() {
                    *create_chat = true;
                }
                if ui.button("↻").on_hover_text("Actualizar").clicked() {
                    *refresh = true;
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("aris_chat_list")
            .show(ui, |ui| {
                for c in &app.aris_chats {
                    let active = c.id == app.active_aris_chat_id;
                    ui.horizontal(|ui| {
                        let label = if c.pinned {
                            format!("📌 {}", c.title)
                        } else {
                            c.title.clone()
                        };
                        if ui.selectable_label(active, label).clicked() {
                            *select_chat = Some(c.id);
                        }
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .small_button("🗑")
                                    .on_hover_text("Eliminar chat")
                                    .clicked()
                                {
                                    *delete_chat = Some(c.id);
                                }
                                if ui
                                    .small_button("📌")
                                    .on_hover_text("Fijar / Desfijar")
                                    .clicked()
                                {
                                    *pin_chat = Some(c.id);
                                }
                            },
                        );
                    });
                }
            });
    });
}

fn show_aris_pane(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    show_back: bool,
    send_prompt: &mut Option<String>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());

        ui.horizontal_wrapped(|ui| {
            if show_back && ui.button("← Chats").clicked() {
                app.mobile_detail_open = false;
            }
            ui.heading("Aris Core");
            if app.offline_mode || app.token.is_none() {
                badge_pill(
                    ui,
                    "Demo local",
                    p.accent_secondary.gamma_multiply(0.18),
                    p.accent_secondary,
                );
            } else {
                badge_pill(
                    ui,
                    &format!(
                        "Créditos: {}/{}",
                        app.aris_credits.remaining, app.aris_credits.daily_limit
                    ),
                    p.success.gamma_multiply(0.18),
                    p.success,
                );
            }
            for mode in [ArisMode::Instantaneo, ArisMode::Rapido, ArisMode::Pensando] {
                if ui
                    .selectable_label(app.aris_mode == mode, mode.label())
                    .clicked()
                {
                    app.aris_mode = mode;
                }
            }
        });

        if let Some(err) = &app.aris_error {
            ui.label(egui::RichText::new(err).small().color(p.danger));
        }
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
                            ui.horizontal_wrapped(|ui| {
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
                if app.aris_busy {
                    ui.label(
                        egui::RichText::new("✨ Aris está pensando…")
                            .italics()
                            .color(p.accent_primary),
                    );
                }
            });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let enter = ui
                .add(
                    egui::TextEdit::singleline(&mut app.aris_draft)
                        .desired_width((ui.available_width() - 105.0).max(120.0))
                        .hint_text("Pregúntale a Aris…"),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));

            let label = if app.aris_busy {
                "Pensando…"
            } else {
                "Enviar"
            };
            if (primary_button(ui, p, label).clicked() || enter) && !app.aris_busy {
                let prompt = app.aris_draft.trim().to_string();
                if !prompt.is_empty() {
                    *send_prompt = Some(prompt);
                    app.aris_draft.clear();
                }
            }
        });
    });
}
