use aethravox_core::{CreateCommunityGroupRequest, CreateCommunityRequest};

use crate::api::{
    spawn_community_create_group, spawn_community_detail, spawn_community_invite,
    spawn_community_inviteable, spawn_community_membership, spawn_create_community,
    spawn_fetch_communities,
};
use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut select_comm: Option<i64> = None;
    let mut toggle_membership: Option<(i64, bool)> = None;
    let mut open_group_chat: Option<i64> = None;
    let mut create_comm = false;
    let mut create_group_for: Option<i64> = None;
    let mut invite_for: Option<i64> = None;

    ui.horizontal(|ui| {
        ui.heading("🌐 Comunidades");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Actualizar").clicked() {
                refresh = true;
            }
        });
    });

    if let Some(err) = &app.communities_error {
        ui.label(egui::RichText::new(err).small().color(p.danger));
    }
    ui.add_space(8.0);

    let mobile = app.is_mobile_layout(&ctx);

    if mobile {
        if !app.mobile_detail_open {
            show_comm_list(
                app,
                ui,
                p,
                ui.available_width(),
                &mut create_comm,
                &mut select_comm,
            );
        } else {
            show_comm_detail(
                app,
                ui,
                p,
                true,
                &mut toggle_membership,
                &mut open_group_chat,
                &mut create_group_for,
                &mut invite_for,
            );
        }
    } else {
        ui.horizontal(|ui| {
            show_comm_list(
                app,
                ui,
                p,
                280.0,
                &mut create_comm,
                &mut select_comm,
            );
            ui.add_space(10.0);
            show_comm_detail(
                app,
                ui,
                p,
                false,
                &mut toggle_membership,
                &mut open_group_chat,
                &mut create_group_for,
                &mut invite_for,
            );
        });
    }

    if refresh && !app.offline_mode && app.token.is_some() {
        spawn_fetch_communities(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some(id) = select_comm {
        app.mobile_detail_open = true;
        if !app.offline_mode && app.token.is_some() {
            spawn_community_detail(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                id,
            );
            spawn_community_inviteable(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                id,
            );
        }
    }
    if let Some((id, join)) = toggle_membership {
        if !app.offline_mode && app.token.is_some() {
            spawn_community_membership(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                id,
                join,
            );
        }
    }
    if create_comm && !app.offline_mode && app.token.is_some() {
        let req = CreateCommunityRequest {
            name: app.community_new_name.trim().to_string(),
            description: Some(app.community_new_desc.trim().to_string()),
            icon: Some(app.community_new_icon.trim().to_string()),
        };
        app.community_new_name.clear();
        app.community_new_desc.clear();
        app.community_new_icon.clear();
        spawn_create_community(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            req,
        );
    }
    if let Some(cid) = create_group_for {
        if !app.offline_mode && app.token.is_some() {
            let req = CreateCommunityGroupRequest {
                name: app.community_group_name.trim().to_string(),
                description: Some(app.community_group_desc.trim().to_string()),
            };
            app.community_group_name.clear();
            app.community_group_desc.clear();
            spawn_community_create_group(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cid,
                req,
            );
        }
    }
    if let Some(cid) = invite_for {
        if !app.offline_mode && app.token.is_some() {
            let ids = std::mem::take(&mut app.community_invite_selected);
            spawn_community_invite(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cid,
                ids,
            );
        }
    }
    if let Some(gid) = open_group_chat {
        app.route = Route::Chat;
        app.mobile_detail_open = true;
        app.chat_open_conversation(&ctx, gid);
    }
}

fn show_comm_list(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    width: f32,
    create_comm: &mut bool,
    select_comm: &mut Option<i64>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(width);
        ui.set_min_height(ui.available_height());

        ui.collapsing("+ Crear comunidad", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.community_new_name)
                    .hint_text("Nombre de la comunidad"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.community_new_icon)
                    .hint_text("Icono emoji (ej. 🌐)"),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.community_new_desc)
                    .desired_rows(2)
                    .hint_text("Descripción…"),
            );
            if primary_button(ui, p, "Crear").clicked() && !app.community_new_name.trim().is_empty()
            {
                *create_comm = true;
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("comm_list")
            .show(ui, |ui| {
                if app.communities.is_empty() {
                    ui.label(
                        egui::RichText::new("No hay comunidades disponibles.")
                            .small()
                            .color(p.text_muted),
                    );
                }
                let selected_id = app.active_community.as_ref().map(|d| d.community.id);
                for c in &app.communities {
                    let active = selected_id == Some(c.id);
                    let resp = elevated_frame(p)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if active { p.accent_primary } else { p.border_subtle },
                        ))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&c.icon).size(18.0));
                                ui.label(egui::RichText::new(&c.name).strong());
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{} miembros", c.member_count))
                                        .small()
                                        .color(p.text_muted),
                                );
                                if c.is_member {
                                    badge_pill(
                                        ui,
                                        "Miembro",
                                        p.success.gamma_multiply(0.18),
                                        p.success,
                                    );
                                }
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        *select_comm = Some(c.id);
                    }
                    ui.add_space(6.0);
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn show_comm_detail(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    show_back: bool,
    toggle_membership: &mut Option<(i64, bool)>,
    open_group_chat: &mut Option<i64>,
    create_group_for: &mut Option<i64>,
    invite_for: &mut Option<i64>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());

        if show_back {
            if ui.button("← Comunidades").clicked() {
                app.mobile_detail_open = false;
            }
            ui.add_space(4.0);
        }

        if let Some(detail) = app.active_community.clone() {
            egui::ScrollArea::vertical()
                .id_salt("comm_detail")
                .show(ui, |ui| {
                    let c = &detail.community;
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(format!("{}  {}", c.icon, c.name));
                        if c.is_member {
                            if !c.is_owner && ui.button("Salir").clicked() {
                                *toggle_membership = Some((c.id, false));
                            }
                        } else if primary_button(ui, p, "Unirse").clicked() {
                            *toggle_membership = Some((c.id, true));
                        }
                    });
                    if !c.description.is_empty() {
                        ui.label(egui::RichText::new(&c.description).color(p.text_muted));
                    }
                    ui.separator();

                    ui.heading(format!("Canales / Grupos ({})", detail.groups.len()));
                    ui.add_space(6.0);
                    for g in &detail.groups {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("# {}", g.name)).strong(),
                                    );
                                    if !g.description.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&g.description)
                                                .small()
                                                .color(p.text_muted),
                                        );
                                    }
                                });
                                if primary_button(ui, p, "Abrir chat").clicked() {
                                    *open_group_chat = Some(g.id);
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }

                    if c.is_member {
                        ui.add_space(10.0);
                        ui.collapsing("+ Crear canal / grupo", |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.community_group_name)
                                    .hint_text("Nombre del canal"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.community_group_desc)
                                    .hint_text("Descripción opcional"),
                            );
                            if primary_button(ui, p, "Crear grupo").clicked()
                                && !app.community_group_name.trim().is_empty()
                            {
                                *create_group_for = Some(c.id);
                            }
                        });

                        ui.add_space(8.0);
                        ui.collapsing("Invitar amigos", |ui| {
                            if app.community_inviteable.is_empty() {
                                ui.label(
                                    egui::RichText::new("No hay amigos pendientes de unirse.")
                                        .small()
                                        .color(p.text_muted),
                                );
                            } else {
                                for f in &app.community_inviteable {
                                    let mut checked =
                                        app.community_invite_selected.contains(&f.id);
                                    if ui
                                        .checkbox(
                                            &mut checked,
                                            format!("{} (@{})", f.display, f.username),
                                        )
                                        .changed()
                                    {
                                        if checked {
                                            if !app.community_invite_selected.contains(&f.id) {
                                                app.community_invite_selected.push(f.id);
                                            }
                                        } else {
                                            app.community_invite_selected
                                                .retain(|&id| id != f.id);
                                        }
                                    }
                                }
                                if !app.community_invite_selected.is_empty()
                                    && primary_button(ui, p, "Enviar invitaciones").clicked()
                                {
                                    *invite_for = Some(c.id);
                                }
                            }
                        });
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Selecciona una comunidad para ver sus grupos.")
                        .color(p.text_muted),
                );
            });
        }
    });
}
