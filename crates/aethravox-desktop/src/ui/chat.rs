use aethravox_core::{
    ChatGroupManageDto, CreateGroupRankRequest, CreateGroupRequest, UpdateGroupInfoRequest,
};
use egui::Rounding;

use crate::api::{
    spawn_chat_bulk_delete, spawn_chat_group_add_member, spawn_chat_group_assign_rank,
    spawn_chat_group_create_rank, spawn_chat_group_delete_rank, spawn_chat_group_image_bytes,
    spawn_chat_group_image_path, spawn_chat_group_leave, spawn_chat_group_manage,
    spawn_chat_group_remove_member, spawn_chat_group_transfer, spawn_chat_group_update_info,
    spawn_chat_message_info, spawn_chat_upload_media_bytes, spawn_chat_upload_media_path,
    spawn_create_group, spawn_delete_message, spawn_edit_message, spawn_fetch_public_profile,
    spawn_start_dm,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, parse_hex_color, primary_button, Palette};

enum GroupAction {
    FetchManage(i64),
    UpdateInfo {
        conv_id: i64,
        req: UpdateGroupInfoRequest,
    },
    AddMember {
        conv_id: i64,
        username: String,
    },
    RemoveMember {
        conv_id: i64,
        user_id: i64,
    },
    AssignRank {
        conv_id: i64,
        user_id: i64,
        rank_id: i64,
    },
    CreateRank {
        conv_id: i64,
        req: CreateGroupRankRequest,
    },
    DeleteRank {
        conv_id: i64,
        rank_id: i64,
    },
    TransferOwner {
        conv_id: i64,
        new_owner_id: i64,
    },
    LeaveGroup(i64),
}

#[allow(dead_code)]
enum ChatMediaAction {
    UploadBytes {
        conv_id: i64,
        is_image: bool,
        filename: String,
        bytes: Vec<u8>,
    },
    UploadPath {
        conv_id: i64,
        is_image: bool,
        path: String,
    },
    GroupImageBytes {
        conv_id: i64,
        filename: String,
        bytes: Vec<u8>,
    },
    GroupImagePath {
        conv_id: i64,
        path: String,
    },
}

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut clicked_conv: Option<i64> = None;
    let mut send_requested = false;
    let mut start_dm: Option<String> = None;
    let mut create_group: Option<CreateGroupRequest> = None;
    let mut edit_msg: Option<(i64, String)> = None;
    let mut delete_msg: Option<i64> = None;
    let mut info_msg: Option<i64> = None;
    let mut bulk_delete_ids: Option<Vec<i64>> = None;
    let mut group_action: Option<GroupAction> = None;
    let mut media_action: Option<ChatMediaAction> = None;
    let mut view_profile_user: Option<String> = None;
    let mut open_media_url: Option<String> = None;

    if app.active_conv_id != 0 {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(f) = dropped.into_iter().next() {
            if app.chat_group_panel {
                if let Some(bytes) = f.bytes {
                    let name = if f.name.is_empty() {
                        "grupo.png".to_string()
                    } else {
                        f.name.clone()
                    };
                    app.chat_group_img_dropped_name = Some(name);
                    app.chat_group_img_dropped_bytes = Some(bytes.to_vec());
                } else if let Some(path) = f.path {
                    app.chat_group_img_path = path.display().to_string();
                    app.chat_group_img_dropped_name = None;
                    app.chat_group_img_dropped_bytes = None;
                }
            } else if app.chat_attach_open {
                if let Some(bytes) = f.bytes {
                    let name = if f.name.is_empty() {
                        if app.chat_attach_is_image {
                            "imagen.png".to_string()
                        } else {
                            "archivo.bin".to_string()
                        }
                    } else {
                        f.name.clone()
                    };
                    app.chat_attach_dropped_name = Some(name);
                    app.chat_attach_dropped_bytes = Some(bytes.to_vec());
                } else if let Some(path) = f.path {
                    app.chat_attach_path = path.display().to_string();
                    app.chat_attach_dropped_name = None;
                    app.chat_attach_dropped_bytes = None;
                }
            }
        }
    }

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
            show_chat_pane(
                app,
                ui,
                p,
                true,
                &mut send_requested,
                &mut edit_msg,
                &mut delete_msg,
                &mut info_msg,
                &mut bulk_delete_ids,
                &mut group_action,
                &mut media_action,
                &mut view_profile_user,
                &mut open_media_url,
            );
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
            show_chat_pane(
                app,
                ui,
                p,
                false,
                &mut send_requested,
                &mut edit_msg,
                &mut delete_msg,
                &mut info_msg,
                &mut bulk_delete_ids,
                &mut group_action,
                &mut media_action,
                &mut view_profile_user,
                &mut open_media_url,
            );
        });
    }

    if let Some(url) = open_media_url {
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }
    if let Some(id) = clicked_conv {
        app.mobile_detail_open = true;
        app.chat_open_conversation(&ctx, id);
    }
    if send_requested {
        app.chat_send_draft(&ctx);
    }
    if let Some((msg_id, new_body)) = edit_msg {
        if app.offline_mode || app.token.is_none() {
            if let Some(m) = app.chat_messages.iter_mut().find(|m| m.id == msg_id) {
                m.content = format!("{new_body} (editado)");
            }
        } else {
            spawn_edit_message(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                app.active_conv_id,
                msg_id,
                new_body,
            );
        }
    }
    if let Some(msg_id) = delete_msg {
        if app.offline_mode || app.token.is_none() {
            if let Some(m) = app.chat_messages.iter_mut().find(|m| m.id == msg_id) {
                m.content = "🚫 Mensaje eliminado".to_string();
            }
        } else {
            spawn_delete_message(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                app.active_conv_id,
                msg_id,
            );
        }
    }
    if let Some(msg_id) = info_msg {
        if app.offline_mode || app.token.is_none() {
            app.status_banner =
                Some("Información detallada de mensajes disponible en modo conectado.".into());
        } else {
            spawn_chat_message_info(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                msg_id,
            );
        }
    }
    if let Some(ids) = bulk_delete_ids {
        if app.offline_mode || app.token.is_none() {
            for m in &mut app.chat_messages {
                if ids.contains(&m.id) {
                    m.content = "🚫 Mensaje eliminado".to_string();
                }
            }
            app.chat_select_mode = false;
            app.chat_selected_ids.clear();
        } else {
            spawn_chat_bulk_delete(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                ids,
            );
        }
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
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(username) = view_profile_user {
        if app.offline_mode || app.token.is_none() {
            app.status_banner = Some("Conecta al servidor para ver perfiles públicos.".into());
        } else {
            spawn_fetch_public_profile(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                username,
            );
        }
    }
    if let Some(act) = media_action {
        if app.offline_mode || app.token.is_none() {
            app.status_banner =
                Some("La subida de archivos e imágenes requiere conexión con el servidor.".into());
        } else {
            match act {
                ChatMediaAction::UploadBytes {
                    conv_id,
                    is_image,
                    filename,
                    bytes,
                } => {
                    app.chat_attach_busy = true;
                    spawn_chat_upload_media_bytes(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        is_image,
                        filename,
                        String::new(),
                        bytes,
                    );
                }
                ChatMediaAction::UploadPath {
                    conv_id,
                    is_image,
                    path,
                } => {
                    app.chat_attach_busy = true;
                    spawn_chat_upload_media_path(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        is_image,
                        path,
                    );
                }
                ChatMediaAction::GroupImageBytes {
                    conv_id,
                    filename,
                    bytes,
                } => {
                    spawn_chat_group_image_bytes(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        filename,
                        String::new(),
                        bytes,
                    );
                }
                ChatMediaAction::GroupImagePath { conv_id, path } => {
                    spawn_chat_group_image_path(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        path,
                    );
                }
            }
        }
    }
    if let Some(act) = group_action {
        if app.offline_mode || app.token.is_none() {
            app.status_banner =
                Some("Gestión completa de grupos disponible en modo conectado.".into());
        } else {
            match act {
                GroupAction::FetchManage(cid) => {
                    spawn_chat_group_manage(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        cid,
                    );
                }
                GroupAction::UpdateInfo { conv_id, req } => {
                    spawn_chat_group_update_info(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        req,
                    );
                }
                GroupAction::AddMember { conv_id, username } => {
                    spawn_chat_group_add_member(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        username,
                    );
                }
                GroupAction::RemoveMember { conv_id, user_id } => {
                    spawn_chat_group_remove_member(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        user_id,
                    );
                }
                GroupAction::AssignRank {
                    conv_id,
                    user_id,
                    rank_id,
                } => {
                    spawn_chat_group_assign_rank(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        user_id,
                        rank_id,
                    );
                }
                GroupAction::CreateRank { conv_id, req } => {
                    spawn_chat_group_create_rank(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        req,
                    );
                }
                GroupAction::DeleteRank { conv_id, rank_id } => {
                    spawn_chat_group_delete_rank(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        rank_id,
                    );
                }
                GroupAction::TransferOwner {
                    conv_id,
                    new_owner_id,
                } => {
                    spawn_chat_group_transfer(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        conv_id,
                        new_owner_id,
                    );
                }
                GroupAction::LeaveGroup(cid) => {
                    spawn_chat_group_leave(
                        &app.rt,
                        app.tx.clone(),
                        ctx,
                        app.server_url.clone(),
                        app.token.clone(),
                        cid,
                    );
                }
            }
        }
    }
}

fn resolve_media_url(server_url: &str, raw_url: &str) -> String {
    let trimmed = raw_url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with('/') {
        format!("{}{}", server_url.trim_end_matches('/'), trimmed)
    } else {
        format!("{}/{}", server_url.trim_end_matches('/'), trimmed)
    }
}

fn render_message_content(
    ui: &mut egui::Ui,
    p: &Palette,
    server_url: &str,
    content: &str,
    open_media_url: &mut Option<String>,
) {
    if let Some(url) = content
        .strip_prefix("[IMG]")
        .or_else(|| content.strip_prefix("[IMAGE]"))
    {
        elevated_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("📷 Imagen adjunta").strong().small());
                if ui.small_button("Abrir imagen ↗").clicked() {
                    *open_media_url = Some(resolve_media_url(server_url, url));
                }
            });
        });
    } else if let Some(rest) = content.strip_prefix("[VOICE]") {
        let (url, dur) = rest.rsplit_once(':').unwrap_or((rest, ""));
        elevated_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let lbl = if dur.trim().is_empty() {
                    "🎙 Nota de voz".to_string()
                } else {
                    format!("🎙 Nota de voz ({}s)", dur.trim())
                };
                ui.label(egui::RichText::new(lbl).strong().small());
                if ui.small_button("Escuchar ↗").clicked() {
                    *open_media_url = Some(resolve_media_url(server_url, url));
                }
            });
        });
    } else if let Some(rest) = content.strip_prefix("[FILE]") {
        let parts: Vec<&str> = rest.splitn(4, ':').collect();
        let url = parts.first().copied().unwrap_or(rest);
        let name = parts
            .get(1)
            .copied()
            .filter(|s| !s.is_empty())
            .unwrap_or("Archivo");
        let size_label = parts
            .get(3)
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|b| {
                if b >= 1024 * 1024 {
                    format!(" ({:.1} MB)", b as f64 / (1024.0 * 1024.0))
                } else {
                    format!(" ({} KB)", (b / 1024).max(1))
                }
            })
            .unwrap_or_default();
        elevated_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("📎 {name}{size_label}")).strong().small());
                if ui.small_button("Descargar ↗").clicked() {
                    *open_media_url = Some(resolve_media_url(server_url, url));
                }
            });
        });
    } else {
        ui.label(content);
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

#[allow(clippy::too_many_arguments)]
fn show_chat_pane(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    show_back: bool,
    send_requested: &mut bool,
    edit_msg: &mut Option<(i64, String)>,
    delete_msg: &mut Option<i64>,
    info_msg: &mut Option<i64>,
    bulk_delete_ids: &mut Option<Vec<i64>>,
    group_action: &mut Option<GroupAction>,
    media_action: &mut Option<ChatMediaAction>,
    view_profile_user: &mut Option<String>,
    open_media_url: &mut Option<String>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());

        let active_conv = app
            .conversations
            .iter()
            .find(|c| c.id == app.active_conv_id)
            .cloned();
        let active_title = active_conv
            .as_ref()
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Conversación".to_string());
        let is_group = active_conv.as_ref().map(|c| c.is_group).unwrap_or(false);

        ui.horizontal_wrapped(|ui| {
            if show_back && ui.button("← Chats").clicked() {
                app.mobile_detail_open = false;
            }
            ui.heading(&active_title);
            if app.active_conv_id != 0 && !(is_group && app.chat_group_panel) {
                if ui
                    .selectable_label(app.chat_select_mode, "☑ Seleccionar")
                    .clicked()
                {
                    app.chat_select_mode = !app.chat_select_mode;
                    if !app.chat_select_mode {
                        app.chat_selected_ids.clear();
                    }
                }
                if app.chat_select_mode && !app.chat_selected_ids.is_empty() {
                    if ui
                        .button(
                            egui::RichText::new(format!(
                                "🗑 Borrar ({})",
                                app.chat_selected_ids.len()
                            ))
                            .color(p.danger),
                        )
                        .clicked()
                    {
                        *bulk_delete_ids = Some(app.chat_selected_ids.clone());
                    }
                    if ui.button("Cancelar").clicked() {
                        app.chat_select_mode = false;
                        app.chat_selected_ids.clear();
                    }
                }
            }
            if is_group {
                let grp_lbl = if app.chat_group_panel {
                    "✕ Cerrar gestión"
                } else {
                    "⚙ Grupo"
                };
                if ui.selectable_label(app.chat_group_panel, grp_lbl).clicked() {
                    app.chat_group_panel = !app.chat_group_panel;
                    if app.chat_group_panel {
                        *group_action = Some(GroupAction::FetchManage(app.active_conv_id));
                    }
                }
            }
        });
        ui.separator();

        if let Some(err) = &app.chat_error {
            ui.label(egui::RichText::new(err).small().color(p.danger));
        }

        if is_group && app.chat_group_panel {
            show_group_manage_panel(
                app,
                ui,
                p,
                group_action,
                media_action,
                view_profile_user,
                open_media_url,
            );
            return;
        }

        let msgs: Vec<_> = app
            .chat_messages
            .iter()
            .filter(|m| m.conversation_id == app.active_conv_id)
            .cloned()
            .collect();

        let footer_reserve = if app.chat_attach_open { 156.0 } else { 56.0 };
        let msg_height = (ui.available_height() - footer_reserve).max(140.0);
        egui::ScrollArea::vertical()
            .id_salt("chat_msgs")
            .max_height(msg_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for m in msgs {
                    let is_selected = app.chat_selected_ids.contains(&m.id);
                    let bg = if is_selected {
                        p.danger.gamma_multiply(0.14)
                    } else if m.is_mine {
                        p.accent_primary.gamma_multiply(0.16)
                    } else {
                        p.bg_elevated
                    };
                    egui::Frame::none()
                        .fill(bg)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if is_selected { p.danger } else { p.border_subtle },
                        ))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if app.chat_select_mode {
                                    let mut checked = is_selected;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if checked {
                                            if !app.chat_selected_ids.contains(&m.id) {
                                                app.chat_selected_ids.push(m.id);
                                            }
                                        } else {
                                            app.chat_selected_ids.retain(|id| *id != m.id);
                                        }
                                    }
                                }
                                ui.label(egui::RichText::new(&m.sender).strong().small());
                                ui.label(
                                    egui::RichText::new(&m.created_at)
                                        .small()
                                        .color(p.text_muted),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("ℹ")
                                            .on_hover_text("Info del mensaje")
                                            .clicked()
                                        {
                                            *info_msg = Some(m.id);
                                        }
                                        if m.is_mine && m.content != "🚫 Mensaje eliminado" {
                                            if ui
                                                .small_button(
                                                    egui::RichText::new("🗑").color(p.danger),
                                                )
                                                .on_hover_text("Eliminar mensaje")
                                                .clicked()
                                            {
                                                *delete_msg = Some(m.id);
                                                if app.chat_editing_id == Some(m.id) {
                                                    app.chat_editing_id = None;
                                                    app.chat_edit_draft.clear();
                                                }
                                            }
                                            if ui
                                                .small_button("✏")
                                                .on_hover_text("Editar mensaje")
                                                .clicked()
                                            {
                                                app.chat_editing_id = Some(m.id);
                                                app.chat_edit_draft = m.content.clone();
                                            }
                                        }
                                    },
                                );
                            });

                            if app.chat_editing_id == Some(m.id) {
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.chat_edit_draft)
                                            .desired_width((ui.available_width() - 150.0).max(120.0)),
                                    );
                                    if primary_button(ui, p, "Guardar").clicked()
                                        && !app.chat_edit_draft.trim().is_empty()
                                    {
                                        *edit_msg =
                                            Some((m.id, app.chat_edit_draft.trim().to_string()));
                                        app.chat_editing_id = None;
                                        app.chat_edit_draft.clear();
                                    }
                                    if ui.button("Cancelar").clicked() {
                                        app.chat_editing_id = None;
                                        app.chat_edit_draft.clear();
                                    }
                                });
                            } else {
                                render_message_content(
                                    ui,
                                    p,
                                    &app.server_url,
                                    &m.content,
                                    open_media_url,
                                );
                            }
                        });
                    ui.add_space(6.0);
                }
            });

        if app.chat_attach_open && app.active_conv_id != 0 {
            ui.add_space(4.0);
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(app.chat_attach_is_image, "🖼 Imagen (máx 5 MB)")
                        .clicked()
                    {
                        app.chat_attach_is_image = true;
                    }
                    if ui
                        .selectable_label(!app.chat_attach_is_image, "📄 Archivo (máx 20 MB)")
                        .clicked()
                    {
                        app.chat_attach_is_image = false;
                    }
                    if let Some(name) = app.chat_attach_dropped_name.clone() {
                        badge_pill(
                            ui,
                            &format!("Listo: {name}"),
                            p.accent_secondary.gamma_multiply(0.20),
                            p.accent_secondary,
                        );
                        if ui.small_button("✕").clicked() {
                            app.chat_attach_dropped_name = None;
                            app.chat_attach_dropped_bytes = None;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    ui.add(
                        egui::TextEdit::singleline(&mut app.chat_attach_path)
                            .desired_width((ui.available_width() - 150.0).max(140.0))
                            .hint_text("Ruta local o arrastra archivo aquí…"),
                    );
                    #[cfg(target_arch = "wasm32")]
                    ui.label(
                        egui::RichText::new("Arrastra un archivo a esta ventana para adjuntarlo.")
                            .small()
                            .color(p.text_muted),
                    );
                    let send_lbl = if app.chat_attach_busy {
                        "Subiendo…"
                    } else {
                        "Enviar adjunto"
                    };
                    if primary_button(ui, p, send_lbl).clicked() && !app.chat_attach_busy {
                        if let Some(bytes) = app.chat_attach_dropped_bytes.take() {
                            let name = app.chat_attach_dropped_name.take().unwrap_or_else(|| {
                                if app.chat_attach_is_image {
                                    "imagen.png".to_string()
                                } else {
                                    "archivo.bin".to_string()
                                }
                            });
                            *media_action = Some(ChatMediaAction::UploadBytes {
                                conv_id: app.active_conv_id,
                                is_image: app.chat_attach_is_image,
                                filename: name,
                                bytes,
                            });
                        } else {
                            #[cfg(not(target_arch = "wasm32"))]
                            if !app.chat_attach_path.trim().is_empty() {
                                *media_action = Some(ChatMediaAction::UploadPath {
                                    conv_id: app.active_conv_id,
                                    is_image: app.chat_attach_is_image,
                                    path: app.chat_attach_path.trim().to_string(),
                                });
                            }
                        }
                    }
                });
            });
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if app.active_conv_id != 0
                && ui
                    .selectable_label(app.chat_attach_open, "📎")
                    .on_hover_text("Adjuntar imagen o archivo")
                    .clicked()
            {
                app.chat_attach_open = !app.chat_attach_open;
            }
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.chat_draft)
                    .desired_width((ui.available_width() - 105.0).max(110.0))
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

fn show_group_manage_panel(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    group_action: &mut Option<GroupAction>,
    media_action: &mut Option<ChatMediaAction>,
    view_profile_user: &mut Option<String>,
    open_media_url: &mut Option<String>,
) {
    let dto: ChatGroupManageDto = match &app.chat_group_manage {
        Some(d) => d.clone(),
        None => {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(if app.offline_mode || app.token.is_none() {
                    "La gestión de miembros, rangos y ownership requiere conexión con el servidor."
                } else {
                    "Cargando gestión del grupo…"
                })
                .small()
                .color(p.text_muted),
            );
            return;
        }
    };

    egui::ScrollArea::vertical()
        .id_salt("chat_group_manage_scroll")
        .show(ui, |ui| {
            // Información del grupo
            elevated_frame(p).show(ui, |ui| {
                ui.label(egui::RichText::new("Información del grupo").strong());
                if !dto.image_url.is_empty() {
                    if ui.small_button("🖼 Ver imagen del grupo ↗").clicked() {
                        *open_media_url = Some(resolve_media_url(&app.server_url, &dto.image_url));
                    }
                }
                if dto.can_edit_group_info {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.chat_group_edit_name)
                            .desired_width(ui.available_width().min(320.0))
                            .hint_text("Nombre del grupo"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.chat_group_edit_desc)
                            .desired_width(ui.available_width().min(420.0))
                            .hint_text("Descripción del grupo"),
                    );
                    if primary_button(ui, p, "💾 Guardar info").clicked()
                        && !app.chat_group_edit_name.trim().is_empty()
                    {
                        *group_action = Some(GroupAction::UpdateInfo {
                            conv_id: dto.conversation_id,
                            req: UpdateGroupInfoRequest {
                                name: app.chat_group_edit_name.trim().to_string(),
                                description: Some(app.chat_group_edit_desc.trim().to_string()),
                            },
                        });
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("Imagen del grupo").strong().small());
                    if let Some(name) = app.chat_group_img_dropped_name.clone() {
                        ui.horizontal_wrapped(|ui| {
                            badge_pill(
                                ui,
                                &format!("Lista: {name}"),
                                p.accent_secondary.gamma_multiply(0.20),
                                p.accent_secondary,
                            );
                            if ui.small_button("✕").clicked() {
                                app.chat_group_img_dropped_name = None;
                                app.chat_group_img_dropped_bytes = None;
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.add(
                            egui::TextEdit::singleline(&mut app.chat_group_img_path)
                                .desired_width(ui.available_width().min(280.0))
                                .hint_text("Ruta de imagen o arrástrala aquí…"),
                        );
                        #[cfg(target_arch = "wasm32")]
                        ui.label(
                            egui::RichText::new("Arrastra una imagen aquí para cambiar el icono.")
                                .small()
                                .color(p.text_muted),
                        );
                        if ui.button("📷 Cambiar imagen").clicked() {
                            if let Some(bytes) = app.chat_group_img_dropped_bytes.take() {
                                let name = app
                                    .chat_group_img_dropped_name
                                    .take()
                                    .unwrap_or_else(|| "grupo.png".to_string());
                                *media_action = Some(ChatMediaAction::GroupImageBytes {
                                    conv_id: dto.conversation_id,
                                    filename: name,
                                    bytes,
                                });
                            } else {
                                #[cfg(not(target_arch = "wasm32"))]
                                if !app.chat_group_img_path.trim().is_empty() {
                                    *media_action = Some(ChatMediaAction::GroupImagePath {
                                        conv_id: dto.conversation_id,
                                        path: app.chat_group_img_path.trim().to_string(),
                                    });
                                }
                            }
                        }
                    });
                } else {
                    ui.label(egui::RichText::new(&dto.name).strong());
                    if !dto.description.is_empty() {
                        ui.label(
                            egui::RichText::new(&dto.description)
                                .small()
                                .color(p.text_muted),
                        );
                    }
                }
            });

            ui.add_space(8.0);

            // Miembros
            elevated_frame(p).show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("Miembros ({})", dto.members.len())).strong(),
                );
                if dto.can_add_members {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.chat_group_add_user)
                                .desired_width(ui.available_width().min(220.0))
                                .hint_text("@usuario a añadir"),
                        );
                        if primary_button(ui, p, "➕ Añadir").clicked()
                            && !app.chat_group_add_user.trim().is_empty()
                        {
                            *group_action = Some(GroupAction::AddMember {
                                conv_id: dto.conversation_id,
                                username: app.chat_group_add_user.trim().to_string(),
                            });
                            app.chat_group_add_user.clear();
                        }
                    });
                    ui.add_space(4.0);
                }

                for m in &dto.members {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&m.display).strong().small());
                        ui.label(
                            egui::RichText::new(format!("@{}", m.username))
                                .small()
                                .color(p.text_muted),
                        );
                        if ui
                            .small_button("👤")
                            .on_hover_text("Ver perfil público")
                            .clicked()
                        {
                            *view_profile_user = Some(m.username.clone());
                        }
                        let rcol = parse_hex_color(&m.rank_color, p.accent_secondary);
                        badge_pill(ui, &m.rank_name, rcol.gamma_multiply(0.18), rcol);
                        if m.is_owner {
                            badge_pill(
                                ui,
                                "👑 Owner",
                                p.accent_primary.gamma_multiply(0.20),
                                p.accent_primary,
                            );
                        }

                        if dto.can_assign_ranks && !m.is_owner {
                            let mut chosen = m.rank_id;
                            egui::ComboBox::from_id_salt(("grp_rank_cb", m.user_id))
                                .selected_text(&m.rank_name)
                                .show_ui(ui, |ui| {
                                    for r in dto.ranks.iter().filter(|r| !r.is_owner_rank) {
                                        if ui
                                            .selectable_value(&mut chosen, r.id, &r.name)
                                            .clicked()
                                            && r.id != m.rank_id
                                        {
                                            *group_action = Some(GroupAction::AssignRank {
                                                conv_id: dto.conversation_id,
                                                user_id: m.user_id,
                                                rank_id: r.id,
                                            });
                                        }
                                    }
                                });
                        }

                        if dto.can_transfer_ownership
                            && !m.is_owner
                            && ui
                                .small_button("👑 Ceder")
                                .on_hover_text("Transferir ownership")
                                .clicked()
                        {
                            *group_action = Some(GroupAction::TransferOwner {
                                conv_id: dto.conversation_id,
                                new_owner_id: m.user_id,
                            });
                        }

                        if dto.can_remove_members
                            && !m.is_owner
                            && ui
                                .small_button(egui::RichText::new("✕").color(p.danger))
                                .on_hover_text("Expulsar del grupo")
                                .clicked()
                        {
                            *group_action = Some(GroupAction::RemoveMember {
                                conv_id: dto.conversation_id,
                                user_id: m.user_id,
                            });
                        }
                    });
                }
            });

            ui.add_space(8.0);

            // Rangos
            elevated_frame(p).show(ui, |ui| {
                ui.label(egui::RichText::new(format!("Rangos ({})", dto.ranks.len())).strong());
                for r in &dto.ranks {
                    ui.horizontal_wrapped(|ui| {
                        let rcol = parse_hex_color(&r.color, p.accent_primary);
                        badge_pill(ui, &r.name, rcol.gamma_multiply(0.18), rcol);
                        let mut perms = Vec::new();
                        if r.can_send_messages {
                            perms.push("enviar");
                        }
                        if r.can_add_members {
                            perms.push("añadir");
                        }
                        if r.can_edit_group_info {
                            perms.push("editar");
                        }
                        if r.can_remove_members {
                            perms.push("expulsar");
                        }
                        if r.can_manage_ranks {
                            perms.push("rangos");
                        }
                        if r.can_assign_ranks {
                            perms.push("asignar");
                        }
                        if r.can_transfer_ownership {
                            perms.push("transferir");
                        }
                        ui.label(
                            egui::RichText::new(perms.join(" · "))
                                .small()
                                .color(p.text_muted),
                        );
                        if dto.can_manage_ranks
                            && !r.is_owner_rank
                            && ui
                                .small_button(egui::RichText::new("🗑").color(p.danger))
                                .on_hover_text("Eliminar rango")
                                .clicked()
                        {
                            *group_action = Some(GroupAction::DeleteRank {
                                conv_id: dto.conversation_id,
                                rank_id: r.id,
                            });
                        }
                    });
                }

                if dto.can_manage_ranks {
                    ui.separator();
                    ui.label(egui::RichText::new("Crear nuevo rango").strong().small());
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.chat_rank_name)
                                .desired_width(160.0)
                                .hint_text("Nombre del rango"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut app.chat_rank_color)
                                .desired_width(90.0)
                                .hint_text("#00d4ff"),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut app.chat_rank_can_send, "Enviar");
                        ui.checkbox(&mut app.chat_rank_can_add, "Añadir");
                        ui.checkbox(&mut app.chat_rank_can_edit_info, "Editar info");
                        ui.checkbox(&mut app.chat_rank_can_delete, "Borrar msgs");
                        ui.checkbox(&mut app.chat_rank_can_remove, "Expulsar");
                        ui.checkbox(&mut app.chat_rank_can_manage, "Gestionar rangos");
                        ui.checkbox(&mut app.chat_rank_can_assign, "Asignar rangos");
                        ui.checkbox(&mut app.chat_rank_can_pin, "Fijar msgs");
                        ui.checkbox(&mut app.chat_rank_can_transfer, "Transferir");
                    });
                    if primary_button(ui, p, "➕ Crear rango").clicked()
                        && !app.chat_rank_name.trim().is_empty()
                    {
                        *group_action = Some(GroupAction::CreateRank {
                            conv_id: dto.conversation_id,
                            req: CreateGroupRankRequest {
                                name: app.chat_rank_name.trim().to_string(),
                                color: Some(app.chat_rank_color.trim().to_string()),
                                can_send_messages: app.chat_rank_can_send,
                                can_add_members: app.chat_rank_can_add,
                                can_edit_group_info: app.chat_rank_can_edit_info,
                                can_delete_messages: app.chat_rank_can_delete,
                                can_remove_members: app.chat_rank_can_remove,
                                can_manage_ranks: app.chat_rank_can_manage,
                                can_assign_ranks: app.chat_rank_can_assign,
                                can_pin_messages: app.chat_rank_can_pin,
                                can_transfer_ownership: app.chat_rank_can_transfer,
                            },
                        });
                        app.chat_rank_name.clear();
                    }
                }
            });

            ui.add_space(8.0);
            if ui
                .button(egui::RichText::new("🚪 Salir del grupo").color(p.danger))
                .clicked()
            {
                *group_action = Some(GroupAction::LeaveGroup(dto.conversation_id));
            }
        });
}

