use egui::{Color32, Rounding};

use crate::api::{
    spawn_fetch_friends, spawn_friend_action, spawn_search_friends, spawn_send_friend_request,
    spawn_start_dm,
};
use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, card_frame, elevated_frame, parse_hex_color, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut trigger_refresh = false;
    let mut start_dm_user: Option<String> = None;
    let mut friend_act: Option<(&'static str, i64)> = None;
    let mut send_req_user: Option<String> = None;
    let mut run_search = false;

    egui::ScrollArea::vertical().id_salt("friends_scroll").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("👥 Amigos");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻ Actualizar").clicked() {
                    trigger_refresh = true;
                }
            });
        });

        if let Some(err) = &app.friends_error {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(err).small().color(p.danger));
        }

        ui.add_space(10.0);

        // Solicitudes pendientes
        if !app.friends.pending.is_empty() {
            card_frame(p).show(ui, |ui| {
                ui.heading(format!(
                    "Solicitudes pendientes ({})",
                    app.friends.pending.len()
                ));
                ui.add_space(8.0);
                for req in &app.friends.pending {
                    elevated_frame(p).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&req.display).strong());
                            ui.label(
                                egui::RichText::new(format!("@{}", req.username))
                                    .small()
                                    .color(p.text_muted),
                            );
                            if primary_button(ui, p, "Aceptar").clicked() {
                                friend_act = Some(("accept", req.id));
                            }
                            if ui.button("Rechazar").clicked() {
                                friend_act = Some(("reject", req.id));
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            });
            ui.add_space(12.0);
        }

        let mobile = app.is_mobile_layout(&ctx);
        if mobile {
            show_friends_list(app, ui, p, &mut friend_act, &mut start_dm_user);
            ui.add_space(12.0);
            show_friends_add(app, ui, p, &mut send_req_user, &mut run_search);
        } else {
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    show_friends_list(app, ui, p, &mut friend_act, &mut start_dm_user);
                });
                cols[1].vertical(|ui| {
                    show_friends_add(app, ui, p, &mut send_req_user, &mut run_search);
                });
            });
        }
    });

    if trigger_refresh && !app.offline_mode && app.token.is_some() {
        spawn_fetch_friends(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if run_search && !app.offline_mode && app.token.is_some() {
        spawn_search_friends(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            app.friends_search_q.clone(),
        );
    }
    if let Some(username) = send_req_user {
        if !app.offline_mode && app.token.is_some() {
            spawn_send_friend_request(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                username,
            );
        }
    }
    if let Some((act, id)) = friend_act {
        if !app.offline_mode && app.token.is_some() {
            spawn_friend_action(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                act,
                id,
            );
        }
    }
    if let Some(username) = start_dm_user {
        if app.offline_mode || app.token.is_none() {
            app.route = Route::Chat;
        } else {
            spawn_start_dm(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                username,
            );
            app.route = Route::Chat;
        }
    }
}

fn show_friends_list(
    app: &AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    friend_act: &mut Option<(&'static str, i64)>,
    start_dm_user: &mut Option<String>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_min_height(300.0);
        ui.heading(format!("Mis amigos ({})", app.friends.friends.len()));
        ui.separator();

        if app.friends.friends.is_empty() {
            ui.label(
                egui::RichText::new("Todavía no tienes amigos agregados.").color(p.text_muted),
            );
        }

        for f in &app.friends.friends {
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let dot_color = if f.online { p.success } else { p.text_muted };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.5, dot_color);

                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&f.display).strong());
                            let rcol = parse_hex_color(&f.role_color, p.accent_secondary);
                            badge_pill(
                                ui,
                                &f.role_display,
                                rcol.gamma_multiply(0.22),
                                rcol,
                            );
                        });
                        ui.label(
                            egui::RichText::new(format!("@{}", f.username))
                                .small()
                                .color(p.text_muted),
                        );
                    });

                    if primary_button(ui, p, "💬 Chat").clicked() {
                        *start_dm_user = Some(f.username.clone());
                    }
                    if ui
                        .button(egui::RichText::new("Eliminar").color(p.danger))
                        .clicked()
                    {
                        *friend_act = Some(("remove", f.id));
                    }
                });
            });
            ui.add_space(6.0);
        }
    });
}

fn show_friends_add(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    send_req_user: &mut Option<String>,
    run_search: &mut bool,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_min_height(300.0);
        ui.heading("Agregar amigo");
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.friends_direct_user)
                    .desired_width((ui.available_width() - 135.0).max(120.0))
                    .hint_text("Usuario exacto…"),
            );
            if primary_button(ui, p, "Enviar solicitud").clicked() {
                let target = app.friends_direct_user.trim().to_string();
                if !target.is_empty() {
                    *send_req_user = Some(target);
                    app.friends_direct_user.clear();
                }
            }
        });

        ui.add_space(12.0);
        ui.label(egui::RichText::new("Buscar en Aethravox").strong());
        ui.horizontal(|ui| {
            let enter = ui
                .add(
                    egui::TextEdit::singleline(&mut app.friends_search_q)
                        .desired_width((ui.available_width() - 90.0).max(120.0))
                        .hint_text("Nombre o @usuario…"),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Buscar").clicked() || enter {
                *run_search = true;
            }
        });

        ui.add_space(8.0);
        for u in &app.friends_search_results {
            egui::Frame::none()
                .fill(p.bg_elevated)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&u.display).strong());
                        ui.label(
                            egui::RichText::new(format!("@{} · {}", u.username, u.role_display))
                                .small()
                                .color(p.text_muted),
                        );
                        if ui.button("+ Agregar").clicked() {
                            *send_req_user = Some(u.username.clone());
                        }
                    });
                });
            ui.add_space(5.0);
        }

        if app.offline_mode {
            ui.add_space(6.0);
            badge_pill(
                ui,
                "Modo local: conecta el servidor para gestionar amigos reales",
                Color32::from_rgb(35, 30, 50),
                p.text_muted,
            );
        }
    });
}
