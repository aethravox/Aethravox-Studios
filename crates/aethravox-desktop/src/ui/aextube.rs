use crate::api::{
    spawn_delete_video, spawn_fetch_videos, spawn_react_video, spawn_subscribe_channel,
    spawn_view_video,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut react_action: Option<(i64, &'static str)> = None;
    let mut subscribe_author: Option<String> = None;
    let mut delete_id: Option<i64> = None;
    let mut watch_video: Option<(i64, String)> = None;

    let my_id = app.hub.as_ref().map(|h| h.user.id).unwrap_or(0);
    let is_admin = app
        .hub
        .as_ref()
        .map(|h| h.user.can_access_admin)
        .unwrap_or(false);

    egui::ScrollArea::vertical()
        .id_salt("aextube_scroll")
        .show(ui, |ui| {
            card_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("📺 aexTube");
                    for (key, label) in [("all", "Todos"), ("videos", "Vídeos"), ("shorts", "Shorts")]
                    {
                        if ui
                            .selectable_label(app.aextube_filter == key, label)
                            .clicked()
                        {
                            app.aextube_filter = key.to_string();
                        }
                    }
                    if ui.button("↻ Actualizar").clicked() {
                        refresh = true;
                    }
                });
            });

            if let Some(err) = &app.aextube_error {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(err).small().color(p.danger));
            }

            ui.add_space(10.0);
            let list: Vec<_> = app
                .videos
                .iter()
                .filter(|v| match app.aextube_filter.as_str() {
                    "videos" => !v.is_short,
                    "shorts" => v.is_short,
                    _ => true,
                })
                .cloned()
                .collect();

            if list.is_empty() {
                card_frame(p).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("No hay vídeos publicados en esta categoría todavía.")
                            .color(p.text_muted),
                    );
                });
                return;
            }

            for v in list {
                elevated_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(&v.title).strong().size(16.0));
                                if v.is_short {
                                    badge_pill(
                                        ui,
                                        "Short",
                                        p.accent_primary.gamma_multiply(0.18),
                                        p.accent_primary,
                                    );
                                }
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} · {} suscriptores · {} vistas · {}",
                                    v.author, v.channel_subscribers, v.views, v.created_at
                                ))
                                .small()
                                .color(p.text_muted),
                            );
                            if !v.description.is_empty() {
                                ui.label(egui::RichText::new(&v.description).small());
                            }
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                let liked = v.user_reaction.as_deref() == Some("like");
                                let disliked = v.user_reaction.as_deref() == Some("dislike");
                                if ui
                                    .selectable_label(liked, format!("👍 {}", v.likes))
                                    .clicked()
                                {
                                    react_action = Some((v.id, "like"));
                                }
                                if ui
                                    .selectable_label(disliked, format!("👎 {}", v.dislikes))
                                    .clicked()
                                {
                                    react_action = Some((v.id, "dislike"));
                                }
                                let sub_label = if v.subscribed {
                                    "✓ Suscrito"
                                } else {
                                    "Suscribirse"
                                };
                                if ui.selectable_label(v.subscribed, sub_label).clicked() {
                                    subscribe_author = Some(v.author.clone());
                                }
                                if (v.user_id == my_id || is_admin)
                                    && ui
                                        .button(egui::RichText::new("🗑 Eliminar").color(p.danger))
                                        .clicked()
                                {
                                    delete_id = Some(v.id);
                                }
                                if primary_button(ui, p, "▶ Reproducir").clicked() {
                                    watch_video = Some((v.id, v.file_path.clone()));
                                }
                            });
                        });
                    });
                });
                ui.add_space(6.0);
            }
        });

    if refresh && !app.offline_mode {
        spawn_fetch_videos(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some((vid, kind)) = react_action {
        if !app.offline_mode && app.token.is_some() {
            spawn_react_video(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                vid,
                kind,
            );
        }
    }
    if let Some(author) = subscribe_author {
        if !app.offline_mode && app.token.is_some() {
            spawn_subscribe_channel(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                author,
            );
        }
    }
    if let Some(vid) = delete_id {
        if !app.offline_mode && app.token.is_some() {
            spawn_delete_video(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                vid,
            );
        }
    }
    if let Some((vid, path)) = watch_video {
        let base = app.server_url.trim_end_matches('/');
        let url = if path.starts_with("http") {
            path
        } else {
            format!("{base}{path}")
        };
        if !app.offline_mode {
            spawn_view_video(&app.rt, app.server_url.clone(), app.token.clone(), vid);
        }
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }
}
