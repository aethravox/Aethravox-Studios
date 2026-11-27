use crate::api::{
    spawn_aextube_upload_bytes, spawn_aextube_upload_path, spawn_delete_video, spawn_fetch_videos,
    spawn_react_video, spawn_subscribe_channel, spawn_view_video,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

#[allow(dead_code)]
enum UploadAction {
    Bytes {
        title: String,
        description: String,
        is_short: bool,
        filename: String,
        bytes: Vec<u8>,
    },
    Path {
        title: String,
        description: String,
        is_short: bool,
        path: String,
    },
}

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut react_action: Option<(i64, &'static str)> = None;
    let mut subscribe_author: Option<String> = None;
    let mut delete_id: Option<i64> = None;
    let mut watch_video: Option<(i64, String)> = None;
    let mut upload_action: Option<UploadAction> = None;

    if app.aextube_upload_open {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(f) = dropped.into_iter().next() {
            if let Some(bytes) = f.bytes {
                let name = if f.name.is_empty() {
                    "video.mp4".to_string()
                } else {
                    f.name.clone()
                };
                app.aextube_up_dropped_name = Some(name);
                app.aextube_up_dropped_bytes = Some(bytes.to_vec());
            } else if let Some(path) = f.path {
                app.aextube_up_path = path.display().to_string();
                app.aextube_up_dropped_name = None;
                app.aextube_up_dropped_bytes = None;
            }
        }
    }

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
                    let up_lbl = if app.aextube_upload_open {
                        "✕ Cerrar subida"
                    } else {
                        "⬆ Subir vídeo"
                    };
                    if ui
                        .selectable_label(app.aextube_upload_open, up_lbl)
                        .clicked()
                    {
                        app.aextube_upload_open = !app.aextube_upload_open;
                    }
                    if ui.button("↻ Actualizar").clicked() {
                        refresh = true;
                    }
                });
            });

            if app.aextube_upload_open {
                ui.add_space(8.0);
                elevated_frame(p).show(ui, |ui| {
                    ui.label(egui::RichText::new("Subir vídeo o Short a aexTube").strong());
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.aextube_up_title)
                            .desired_width(ui.available_width().min(360.0))
                            .hint_text("Título del vídeo…"),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut app.aextube_up_desc)
                            .desired_rows(2)
                            .desired_width(ui.available_width().min(480.0))
                            .hint_text("Descripción (opcional)…"),
                    );
                    ui.checkbox(&mut app.aextube_up_is_short, "Es un Short");

                    ui.label(
                        egui::RichText::new(
                            "Arrastra aquí un archivo .mp4 / .webm / .mov / .ogv (máx. 20 MB)",
                        )
                        .small()
                        .color(p.accent_secondary),
                    );

                    if let Some(name) = app.aextube_up_dropped_name.clone() {
                        let mb = app
                            .aextube_up_dropped_bytes
                            .as_ref()
                            .map(|b| b.len() as f64 / (1024.0 * 1024.0))
                            .unwrap_or(0.0);
                        ui.horizontal_wrapped(|ui| {
                            badge_pill(
                                ui,
                                &format!("📎 {name} ({mb:.2} MB)"),
                                p.success.gamma_multiply(0.18),
                                p.success,
                            );
                            if ui.small_button("Quitar").clicked() {
                                app.aextube_up_dropped_name = None;
                                app.aextube_up_dropped_bytes = None;
                            }
                        });
                    } else {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.aextube_up_path)
                                    .desired_width(ui.available_width().min(480.0))
                                    .hint_text("O escribe/pega la ruta local del vídeo…"),
                            );
                        }
                    }

                    ui.add_space(4.0);
                    let pub_lbl = if app.aextube_up_busy {
                        "Subiendo…"
                    } else {
                        "📤 Publicar"
                    };
                    if primary_button(ui, p, pub_lbl).clicked() && !app.aextube_up_busy {
                        let title = app.aextube_up_title.trim().to_string();
                        if title.is_empty() {
                            app.aextube_error = Some("El título es obligatorio.".into());
                        } else if app.offline_mode || app.token.is_none() {
                            app.aextube_error = Some(
                                "Inicia sesión conectado al servidor para publicar vídeos.".into(),
                            );
                        } else if let (Some(fname), Some(bytes)) = (
                            app.aextube_up_dropped_name.clone(),
                            app.aextube_up_dropped_bytes.clone(),
                        ) {
                            upload_action = Some(UploadAction::Bytes {
                                title,
                                description: app.aextube_up_desc.trim().to_string(),
                                is_short: app.aextube_up_is_short,
                                filename: fname,
                                bytes,
                            });
                        } else if !app.aextube_up_path.trim().is_empty() {
                            upload_action = Some(UploadAction::Path {
                                title,
                                description: app.aextube_up_desc.trim().to_string(),
                                is_short: app.aextube_up_is_short,
                                path: app.aextube_up_path.trim().to_string(),
                            });
                        } else {
                            app.aextube_error = Some(
                                "Arrastra un archivo de vídeo o indica la ruta local.".into(),
                            );
                        }
                    }
                });
            }

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

    if let Some(act) = upload_action {
        app.aextube_up_busy = true;
        app.aextube_error = None;
        match act {
            UploadAction::Bytes {
                title,
                description,
                is_short,
                filename,
                bytes,
            } => {
                spawn_aextube_upload_bytes(
                    &app.rt,
                    app.tx.clone(),
                    ctx.clone(),
                    app.server_url.clone(),
                    app.token.clone(),
                    title,
                    description,
                    is_short,
                    filename,
                    String::new(),
                    bytes,
                );
            }
            UploadAction::Path {
                title,
                description,
                is_short,
                path,
            } => {
                spawn_aextube_upload_path(
                    &app.rt,
                    app.tx.clone(),
                    ctx.clone(),
                    app.server_url.clone(),
                    app.token.clone(),
                    title,
                    description,
                    is_short,
                    path,
                );
            }
        }
    }
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

