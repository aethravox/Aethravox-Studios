use aethravox_core::{
    AdminAddApiKeyRequest, AdminApisConfigDto, AdminBanRequest, AdminCreateUserRequest,
    AdminCreateWebRequest, AdminSaveProviderRequest, AdminUpdateDocRequest, AdminWebTagRequest,
    CreateIpBanRequest, SaveRoleRequest, SaveTitleRequest, SuspendUserRequest, SystemConfigDto,
};

use crate::api::{
    spawn_admin_add_api_key, spawn_admin_add_api_provider, spawn_admin_apis,
    spawn_admin_assign_role_id, spawn_admin_assign_title, spawn_admin_ban_user,
    spawn_admin_clear_rotation_logs, spawn_admin_config_get, spawn_admin_config_save,
    spawn_admin_create_ip_ban, spawn_admin_create_role, spawn_admin_create_title,
    spawn_admin_create_user, spawn_admin_create_web, spawn_admin_delete_api_key,
    spawn_admin_delete_api_provider, spawn_admin_delete_role, spawn_admin_delete_title,
    spawn_admin_delete_user, spawn_admin_delete_web, spawn_admin_doc_delete,
    spawn_admin_doc_regenerate, spawn_admin_doc_update, spawn_admin_doc_upload_bytes,
    spawn_admin_doc_upload_path, spawn_admin_docs_list, spawn_admin_download_create_bytes,
    spawn_admin_download_create_path, spawn_admin_download_delete, spawn_admin_downloads_list,
    spawn_admin_ip_bans, spawn_admin_overview, spawn_admin_reorder_api_provider,
    spawn_admin_reset_api_cooldowns, spawn_admin_revoke_ip_ban, spawn_admin_revoke_title,
    spawn_admin_roles, spawn_admin_save_apis_config, spawn_admin_set_role,
    spawn_admin_set_role_perms, spawn_admin_set_role_web_perms, spawn_admin_set_web_tag,
    spawn_admin_simple_action, spawn_admin_suspend_user, spawn_admin_test_api_key,
    spawn_admin_titles, spawn_admin_toggle_api_key, spawn_admin_toggle_api_provider,
    spawn_admin_update_api_key_cap, spawn_admin_update_api_provider, spawn_admin_update_role,
    spawn_admin_update_title, spawn_admin_update_web_html, spawn_admin_user_ips,
};
use crate::app::AethravoxApp;
use crate::theme::{
    badge_pill, card_frame, elevated_frame, parse_hex_color, primary_button, Palette,
};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut fetch_ip_bans = false;
    let mut fetch_config = false;
    let mut fetch_roles = false;
    let mut fetch_titles = false;
    let mut role_change: Option<(i64, String)> = None;
    let mut assign_role_id: Option<(i64, i64)> = None;
    let mut simple_action: Option<(String, &'static str)> = None;
    let mut submit_ban: Option<(i64, AdminBanRequest)> = None;
    let mut submit_suspend: Option<(i64, SuspendUserRequest)> = None;
    let mut submit_delete: Option<i64> = None;
    let mut create_ip_ban: Option<CreateIpBanRequest> = None;
    let mut revoke_ip_ban: Option<i64> = None;
    let mut save_config: Option<SystemConfigDto> = None;
    let mut create_role: Option<SaveRoleRequest> = None;
    let mut update_role: Option<(i64, SaveRoleRequest)> = None;
    let mut delete_role: Option<(i64, Option<i64>)> = None;
    let mut save_role_perms: Option<(i64, Vec<String>)> = None;
    let mut save_role_webs: Option<(i64, Vec<i64>)> = None;
    let mut create_title: Option<SaveTitleRequest> = None;
    let mut update_title: Option<(i64, SaveTitleRequest)> = None;
    let mut delete_title: Option<i64> = None;
    let mut assign_title: Option<(i64, i64)> = None;
    let mut revoke_title: Option<(i64, i64)> = None;
    let mut fetch_apis = false;
    let mut fetch_docs = false;
    let mut fetch_downloads = false;
    let mut fetch_user_ips: Option<i64> = None;
    let mut create_user_req: Option<AdminCreateUserRequest> = None;
    let mut create_web_req: Option<AdminCreateWebRequest> = None;
    let mut save_web_html: Option<(i64, String)> = None;
    let mut save_web_tag: Option<(i64, AdminWebTagRequest)> = None;
    let mut delete_web_id: Option<i64> = None;
    let mut add_api_key_req: Option<AdminAddApiKeyRequest> = None;
    let mut toggle_api_key: Option<(i64, bool)> = None;
    let mut cap_api_key: Option<(i64, Option<i64>)> = None;
    let mut test_api_key: Option<i64> = None;
    let mut delete_api_key: Option<i64> = None;
    let mut reset_api_cooldowns = false;
    let mut add_provider_req: Option<AdminSaveProviderRequest> = None;
    let mut update_provider_req: Option<(i64, AdminSaveProviderRequest)> = None;
    let mut toggle_provider: Option<(i64, bool)> = None;
    let mut reorder_provider: Option<(i64, i64)> = None;
    let mut delete_provider: Option<i64> = None;
    let mut save_apis_cfg: Option<AdminApisConfigDto> = None;
    let mut clear_rot_logs = false;
    let mut doc_upload_bytes: Option<(String, Option<String>, String, Vec<u8>)> = None;
    let mut doc_upload_path: Option<(String, Option<String>, String)> = None;
    let mut doc_update: Option<(i64, AdminUpdateDocRequest)> = None;
    let mut doc_regenerate: Option<i64> = None;
    let mut doc_delete: Option<i64> = None;
    let mut dl_upload_bytes: Option<(String, String, String, String, Option<String>, String, Vec<u8>)> = None;
    let mut dl_upload_path: Option<(String, String, String, String, Option<String>, String)> = None;
    let mut dl_delete: Option<i64> = None;

    if matches!(app.admin_tab, 8 | 9) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let fname = if !f.name.is_empty() {
                f.name.clone()
            } else if let Some(path) = &f.path {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("archivo")
                    .to_string()
            } else {
                "archivo".to_string()
            };
            let bytes_opt: Option<Vec<u8>> = if let Some(b) = f.bytes {
                Some(b.to_vec())
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    f.path.as_ref().and_then(|pp| std::fs::read(pp).ok())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
            };
            if let Some(b) = bytes_opt {
                if app.admin_tab == 8 {
                    app.admin_doc_up_dropped_name = Some(fname);
                    app.admin_doc_up_dropped_bytes = Some(b);
                } else if app.admin_tab == 9 {
                    app.admin_dl_dropped_name = Some(fname);
                    app.admin_dl_dropped_bytes = Some(b);
                }
            }
        }
    }

    ui.horizontal(|ui| {
        ui.heading("🛡 Panel de Administración");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Actualizar").clicked() {
                refresh = true;
            }
        });
    });

    if let Some(err) = &app.admin_error {
        ui.label(egui::RichText::new(err).small().color(p.danger));
    }

    ui.add_space(8.0);

    if let Some(ov) = app.admin_overview.clone() {
        ui.horizontal_wrapped(|ui| {
            stat_chip(ui, p, "Usuarios", ov.total_users);
            stat_chip(ui, p, "Owners", ov.owner_count);
            stat_chip(ui, p, "Admins", ov.admin_count);
            stat_chip(ui, p, "Staff", ov.staff_count);
            stat_chip(ui, p, "Secciones", ov.web_count);
            stat_chip(ui, p, "Visitas 7d", ov.visits_7d);
            stat_chip(ui, p, "Auditoría", ov.audit_total);
        });

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            for (idx, label) in [
                (0usize, "Usuarios"),
                (1, "Secciones / Webs"),
                (2, "Auditoría"),
                (3, "🌐 Bans IP"),
                (4, "⚙ Sistema"),
                (5, "🎭 Roles"),
                (6, "🏅 Títulos"),
                (7, "🔑 APIs Aris"),
                (8, "📄 Docs"),
                (9, "⬇ Descargas"),
            ] {
                if ui.selectable_label(app.admin_tab == idx, label).clicked() {
                    app.admin_tab = idx;
                    if idx == 3 && app.admin_ip_bans.is_none() {
                        fetch_ip_bans = true;
                    } else if idx == 4 && app.admin_config.is_none() {
                        fetch_config = true;
                    } else if idx == 5 && app.admin_roles.is_none() {
                        fetch_roles = true;
                    } else if idx == 6 && app.admin_titles.is_none() {
                        fetch_titles = true;
                    } else if idx == 7 && app.admin_apis.is_none() {
                        fetch_apis = true;
                    } else if idx == 8 && app.admin_docs.is_none() {
                        fetch_docs = true;
                    } else if idx == 9 && app.admin_downloads.is_none() {
                        fetch_downloads = true;
                    }
                }
            }
        });
        ui.separator();

        // Modal / barra rápida de baneo
        if let Some((uid, uname)) = app.admin_ban_target.clone() {
            elevated_frame(p).show(ui, |ui| {
                ui.heading(format!("Sancionar a @{uname}"));
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.admin_ban_reason)
                            .desired_width(ui.available_width().min(240.0))
                            .hint_text("Motivo del baneo…"),
                    );
                    ui.checkbox(&mut app.admin_ban_permanent, "Permanente");
                    if !app.admin_ban_permanent {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.admin_ban_hours)
                                .desired_width(70.0)
                                .hint_text("Horas"),
                        );
                    }
                    if primary_button(ui, p, "Confirmar baneo").clicked() {
                        let hours = if app.admin_ban_permanent {
                            None
                        } else {
                            app.admin_ban_hours.trim().parse::<i64>().ok().or(Some(24))
                        };
                        submit_ban = Some((
                            uid,
                            AdminBanRequest {
                                reason: app.admin_ban_reason.trim().to_string(),
                                permanent: app.admin_ban_permanent,
                                duration_hours: hours,
                            },
                        ));
                        app.admin_ban_target = None;
                        app.admin_ban_reason.clear();
                    }
                    if ui.button("Cancelar").clicked() {
                        app.admin_ban_target = None;
                    }
                });
            });
            ui.add_space(8.0);
        }

        // Modal / barra rápida de suspensión
        if let Some((uid, uname)) = app.admin_suspend_target.clone() {
            elevated_frame(p).show(ui, |ui| {
                ui.heading(format!("Suspender a @{uname}"));
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.admin_suspend_reason)
                            .desired_width(ui.available_width().min(240.0))
                            .hint_text("Motivo de la suspensión…"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.admin_suspend_hours)
                            .desired_width(70.0)
                            .hint_text("Horas"),
                    );
                    if primary_button(ui, p, "Confirmar suspensión").clicked() {
                        let hours = app
                            .admin_suspend_hours
                            .trim()
                            .parse::<i64>()
                            .ok()
                            .unwrap_or(24)
                            .max(1);
                        submit_suspend = Some((
                            uid,
                            SuspendUserRequest {
                                reason: app.admin_suspend_reason.trim().to_string(),
                                duration_hours: hours,
                            },
                        ));
                        app.admin_suspend_target = None;
                        app.admin_suspend_reason.clear();
                    }
                    if ui.button("Cancelar").clicked() {
                        app.admin_suspend_target = None;
                    }
                });
            });
            ui.add_space(8.0);
        }

        // Confirmación rápida para eliminar usuario
        if let Some((uid, uname)) = app.admin_delete_confirm.clone() {
            elevated_frame(p).show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "¿Eliminar definitivamente la cuenta @{uname}? Esta acción no se puede deshacer."
                    ))
                    .strong()
                    .color(p.danger),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(egui::RichText::new("Confirmar eliminación").color(p.danger))
                        .clicked()
                    {
                        submit_delete = Some(uid);
                        app.admin_delete_confirm = None;
                    }
                    if ui.button("Cancelar").clicked() {
                        app.admin_delete_confirm = None;
                    }
                });
            });
            ui.add_space(8.0);
        }

        if let Some((ip_uid, ip_uname)) = app.admin_user_ips_target.clone() {
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(format!("🌐 Historial de IPs — @{ip_uname}"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cerrar").clicked() {
                            app.admin_user_ips_target = None;
                            app.admin_user_ips_list.clear();
                        }
                    });
                });
                if app.admin_user_ips_list.is_empty() {
                    ui.label(
                        egui::RichText::new("Sin registros de IP o cargando…")
                            .small()
                            .color(p.text_muted),
                    );
                } else {
                    for h in &app.admin_user_ips_list {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&h.ip).monospace().strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "Usos: {} · Primera: {} · Última: {}",
                                    h.times_used, h.first_seen, h.last_seen
                                ))
                                .small()
                                .color(p.text_muted),
                            );
                            if h.is_first_time {
                                badge_pill(
                                    ui,
                                    "Nueva",
                                    p.accent_primary.gamma_multiply(0.16),
                                    p.accent_primary,
                                );
                            }
                        });
                    }
                }
                let _ = ip_uid;
            });
            ui.add_space(8.0);
        }
        let roles_snapshot = app.admin_roles.clone();
        egui::ScrollArea::vertical()
            .id_salt("admin_tab_scroll")
            .show(ui, |ui| match app.admin_tab {
                0 => {
                    ui.horizontal_wrapped(|ui| {
                        let lbl = if app.admin_new_user_open {
                            "✖ Cerrar alta de usuario"
                        } else {
                            "➕ Crear usuario"
                        };
                        if ui.button(lbl).clicked() {
                            app.admin_new_user_open = !app.admin_new_user_open;
                        }
                    });
                    if app.admin_new_user_open {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("Nuevo usuario").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_new_user_name)
                                        .desired_width(140.0)
                                        .hint_text("Usuario"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_new_user_email)
                                        .desired_width(180.0)
                                        .hint_text("Email"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_new_user_pass)
                                        .password(true)
                                        .desired_width(140.0)
                                        .hint_text("Contraseña"),
                                );
                                egui::ComboBox::from_id_salt("admin_new_user_role_cb")
                                    .selected_text(&app.admin_new_user_role)
                                    .show_ui(ui, |ui| {
                                        for r in ["usuario", "staff", "admin"] {
                                            ui.selectable_value(
                                                &mut app.admin_new_user_role,
                                                r.to_string(),
                                                r,
                                            );
                                        }
                                    });
                                if primary_button(ui, p, "Crear cuenta").clicked()
                                    && !app.admin_new_user_name.trim().is_empty()
                                    && !app.admin_new_user_email.trim().is_empty()
                                    && !app.admin_new_user_pass.is_empty()
                                {
                                    create_user_req = Some(AdminCreateUserRequest {
                                        username: app.admin_new_user_name.trim().to_string(),
                                        email: app.admin_new_user_email.trim().to_string(),
                                        password: app.admin_new_user_pass.clone(),
                                        role: app.admin_new_user_role.clone(),
                                    });
                                    app.admin_new_user_name.clear();
                                    app.admin_new_user_email.clear();
                                    app.admin_new_user_pass.clear();
                                    app.admin_new_user_open = false;
                                }
                            });
                        });
                    }
                    ui.add_space(6.0);
                    for u in &ov.users {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(&u.display).strong());
                                        ui.label(
                                            egui::RichText::new(format!("@{}", u.username))
                                                .small()
                                                .color(p.text_muted),
                                        );
                                        badge_pill(
                                            ui,
                                            &u.role,
                                            p.accent_secondary.gamma_multiply(0.16),
                                            p.accent_secondary,
                                        );
                                        if u.status != "active" {
                                            badge_pill(
                                                ui,
                                                &u.status,
                                                p.danger.gamma_multiply(0.18),
                                                p.danger,
                                            );
                                        }
                                    });
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} · Último login: {}",
                                            u.email, u.last_login
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                });

                                let mut chosen = u.role.clone();
                                egui::ComboBox::from_id_salt(("role_cb", u.id))
                                    .selected_text(&chosen)
                                    .show_ui(ui, |ui| {
                                        for r in ["usuario", "staff", "admin", "owner"] {
                                            if ui
                                                .selectable_value(&mut chosen, r.to_string(), r)
                                                .clicked()
                                            {
                                                role_change = Some((u.id, r.to_string()));
                                            }
                                        }
                                    });

                                if let Some(rp) = &roles_snapshot {
                                    let mut chosen_rid = 0_i64;
                                    egui::ComboBox::from_id_salt(("role_id_cb", u.id))
                                        .selected_text("Rol custom…")
                                        .show_ui(ui, |ui| {
                                            for r in rp.roles.iter().filter(|r| r.role_key != "owner") {
                                                if ui
                                                    .selectable_value(
                                                        &mut chosen_rid,
                                                        r.id,
                                                        &r.name,
                                                    )
                                                    .clicked()
                                                {
                                                    assign_role_id = Some((u.id, r.id));
                                                }
                                            }
                                        });
                                } else if ui
                                    .small_button("🎭")
                                    .on_hover_text("Cargar roles personalizados")
                                    .clicked()
                                {
                                    fetch_roles = true;
                                }

                                if u.status == "banned" || u.status == "suspended" {
                                    if ui.button("Desbanear").clicked() {
                                        simple_action = Some((
                                            format!("/api/v1/admin/users/{}/unban", u.id),
                                            "Usuario desbaneado.",
                                        ));
                                    }
                                } else {
                                    if ui.button("Suspender").clicked() {
                                        app.admin_suspend_target =
                                            Some((u.id, u.username.clone()));
                                    }
                                    if ui
                                        .button(egui::RichText::new("Banear").color(p.danger))
                                        .clicked()
                                    {
                                        app.admin_ban_target = Some((u.id, u.username.clone()));
                                    }
                                }

                                if ui
                                    .button("🌐 IPs")
                                    .on_hover_text("Ver historial de IPs")
                                    .clicked()
                                {
                                    app.admin_user_ips_target = Some((u.id, u.username.clone()));
                                    app.admin_user_ips_list.clear();
                                    fetch_user_ips = Some(u.id);
                                }
                                if ui.button("Forzar logout").clicked() {
                                    simple_action = Some((
                                        format!("/api/v1/admin/users/{}/force_logout", u.id),
                                        "Sesiones revocadas.",
                                    ));
                                }

                                if u.role != "owner"
                                    && ui
                                        .button(egui::RichText::new("🗑").color(p.danger))
                                        .on_hover_text("Eliminar cuenta")
                                        .clicked()
                                {
                                    app.admin_delete_confirm = Some((u.id, u.username.clone()));
                                }
                            });
                        });
                        ui.add_space(5.0);
                    }
                }
                1 => {
                    ui.horizontal_wrapped(|ui| {
                        let lbl = if app.admin_web_create_open {
                            "✖ Cerrar nueva sección"
                        } else {
                            "➕ Crear sección / web"
                        };
                        if ui.button(lbl).clicked() {
                            app.admin_web_create_open = !app.admin_web_create_open;
                        }
                    });
                    if app.admin_web_create_open {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("Nueva sección en el Hub").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_web_new_slug)
                                        .desired_width(120.0)
                                        .hint_text("slug (ej. mi-web)"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_web_new_name)
                                        .desired_width(160.0)
                                        .hint_text("Nombre visible"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_web_new_icon)
                                        .desired_width(50.0)
                                        .hint_text("🧩"),
                                );
                                ui.checkbox(&mut app.admin_web_new_public, "Pública");
                            });
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_web_new_desc)
                                    .desired_width(ui.available_width().min(520.0))
                                    .hint_text("Descripción breve…"),
                            );
                            ui.horizontal(|ui| {
                                ui.label("Tipo:");
                                ui.radio_value(
                                    &mut app.admin_web_new_type,
                                    "external".to_string(),
                                    "URL externa",
                                );
                                ui.radio_value(
                                    &mut app.admin_web_new_type,
                                    "custom".to_string(),
                                    "HTML propio",
                                );
                            });
                            if app.admin_web_new_type == "custom" {
                                ui.add(
                                    egui::TextEdit::multiline(&mut app.admin_web_new_html)
                                        .desired_width(ui.available_width().min(600.0))
                                        .desired_rows(5)
                                        .hint_text("<html><body><h1>Hola</h1></body></html>"),
                                );
                            } else {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_web_new_url)
                                        .desired_width(ui.available_width().min(420.0))
                                        .hint_text("https://… o /ruta"),
                                );
                            }
                            if primary_button(ui, p, "Crear sección").clicked()
                                && !app.admin_web_new_slug.trim().is_empty()
                                && !app.admin_web_new_name.trim().is_empty()
                            {
                                create_web_req = Some(AdminCreateWebRequest {
                                    slug: app.admin_web_new_slug.trim().to_string(),
                                    name: app.admin_web_new_name.trim().to_string(),
                                    description: Some(app.admin_web_new_desc.clone()),
                                    icon: Some(app.admin_web_new_icon.clone()),
                                    url_type: app.admin_web_new_type.clone(),
                                    url: app.admin_web_new_url.trim().to_string(),
                                    html_content: if app.admin_web_new_type == "custom" {
                                        Some(app.admin_web_new_html.clone())
                                    } else {
                                        None
                                    },
                                    is_public: app.admin_web_new_public,
                                });
                                app.admin_web_new_slug.clear();
                                app.admin_web_new_name.clear();
                                app.admin_web_new_desc.clear();
                                app.admin_web_new_url.clear();
                                app.admin_web_new_html.clear();
                                app.admin_web_create_open = false;
                            }
                        });
                    }
                    if let Some((hid, hname)) = app.admin_web_html_target.clone() {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("Editar HTML de \"{hname}\"")).strong(),
                            );
                            ui.add(
                                egui::TextEdit::multiline(&mut app.admin_web_html_draft)
                                    .desired_width(ui.available_width().min(660.0))
                                    .desired_rows(8),
                            );
                            ui.horizontal(|ui| {
                                if primary_button(ui, p, "💾 Guardar HTML").clicked() {
                                    save_web_html = Some((hid, app.admin_web_html_draft.clone()));
                                    app.admin_web_html_target = None;
                                }
                                if ui.button("Cancelar").clicked() {
                                    app.admin_web_html_target = None;
                                }
                            });
                        });
                    }
                    if let Some((tid, tname)) = app.admin_web_tag_target.clone() {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("Etiqueta y estado de \"{tname}\""))
                                    .strong(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt("web_tag_edit_cb")
                                    .selected_text(&app.admin_web_tag_value)
                                    .show_ui(ui, |ui| {
                                        for t in [
                                            "activo",
                                            "nuevo",
                                            "beta",
                                            "en_proceso",
                                            "próximamente",
                                            "mantenimiento",
                                        ] {
                                            ui.selectable_value(
                                                &mut app.admin_web_tag_value,
                                                t.to_string(),
                                                t,
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_web_tag_opens)
                                        .desired_width(160.0)
                                        .hint_text("Apertura YYYY-MM-DDTHH:MM"),
                                );
                            });
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_web_tag_msg)
                                    .desired_width(ui.available_width().min(480.0))
                                    .hint_text("Mensaje personalizado (opcional)…"),
                            );
                            ui.horizontal(|ui| {
                                if primary_button(ui, p, "Guardar etiqueta").clicked() {
                                    save_web_tag = Some((
                                        tid,
                                        AdminWebTagRequest {
                                            tag: Some(app.admin_web_tag_value.clone()),
                                            tag_message: Some(app.admin_web_tag_msg.clone()),
                                            tag_opens_at: Some(app.admin_web_tag_opens.clone()),
                                        },
                                    ));
                                    app.admin_web_tag_target = None;
                                }
                                if ui.button("Cancelar").clicked() {
                                    app.admin_web_tag_target = None;
                                }
                            });
                        });
                    }
                    ui.add_space(6.0);
                    for w in &ov.webs {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(&w.icon).size(18.0));
                                ui.label(egui::RichText::new(&w.name).strong());
                                ui.label(
                                    egui::RichText::new(format!("/{}", w.slug))
                                        .small()
                                        .color(p.text_muted),
                                );
                                badge_pill(
                                    ui,
                                    if w.is_active { "Activa" } else { "Inactiva" },
                                    if w.is_active {
                                        p.success.gamma_multiply(0.18)
                                    } else {
                                        p.danger.gamma_multiply(0.18)
                                    },
                                    if w.is_active { p.success } else { p.danger },
                                );
                                badge_pill(
                                    ui,
                                    &w.tag,
                                    p.accent_primary.gamma_multiply(0.14),
                                    p.accent_primary,
                                );
                                if !w.is_core {
                                    let label = if w.is_active {
                                        "Desactivar"
                                    } else {
                                        "Activar"
                                    };
                                    if ui.button(label).clicked() {
                                        simple_action = Some((
                                            format!("/api/v1/admin/webs/{}/toggle", w.id),
                                            "Estado de sección actualizado.",
                                        ));
                                    }
                                }
                                if ui.button("🏷 Etiqueta").clicked() {
                                    app.admin_web_tag_target = Some((w.id, w.name.clone()));
                                    app.admin_web_tag_value = if w.tag_raw.is_empty() {
                                        "activo".to_string()
                                    } else {
                                        w.tag_raw.clone()
                                    };
                                    app.admin_web_tag_msg = w.tag_message.clone();
                                    app.admin_web_tag_opens = w.tag_opens_at.clone();
                                }
                                if w.is_custom && ui.button("</> HTML").clicked() {
                                    app.admin_web_html_target = Some((w.id, w.name.clone()));
                                    app.admin_web_html_draft = w.html_content.clone();
                                }
                                if w.is_custom
                                    && !w.is_system
                                    && ui
                                        .button(egui::RichText::new("🗑").color(p.danger))
                                        .on_hover_text("Eliminar sección")
                                        .clicked()
                                {
                                    delete_web_id = Some(w.id);
                                }
                            });
                        });
                        ui.add_space(5.0);
                    }
                }
                2 => {
                    for a in &ov.audit {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(&a.created_at)
                                        .small()
                                        .color(p.text_muted),
                                );
                                ui.label(egui::RichText::new(&a.username).strong().small());
                                badge_pill(
                                    ui,
                                    &a.action,
                                    p.accent_primary.gamma_multiply(0.16),
                                    p.accent_primary,
                                );
                                ui.label(egui::RichText::new(&a.detail).small());
                            });
                        });
                        ui.add_space(4.0);
                    }
                }
                3 => {
                    elevated_frame(p).show(ui, |ui| {
                        ui.label(egui::RichText::new("Crear ban de IP").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_ip_ban_ip)
                                    .desired_width(160.0)
                                    .hint_text("Dirección IP (ej. 1.2.3.4)"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_ip_ban_reason)
                                    .desired_width(220.0)
                                    .hint_text("Motivo del ban…"),
                            );
                            ui.checkbox(&mut app.admin_ip_ban_perm, "Permanente");
                            if !app.admin_ip_ban_perm {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_ip_ban_hours)
                                        .desired_width(70.0)
                                        .hint_text("Horas"),
                                );
                            }
                            if primary_button(ui, p, "Banear IP").clicked()
                                && !app.admin_ip_ban_ip.trim().is_empty()
                                && !app.admin_ip_ban_reason.trim().is_empty()
                            {
                                let hours = if app.admin_ip_ban_perm {
                                    None
                                } else {
                                    Some(
                                        app.admin_ip_ban_hours
                                            .trim()
                                            .parse::<i64>()
                                            .ok()
                                            .unwrap_or(24)
                                            .max(1),
                                    )
                                };
                                create_ip_ban = Some(CreateIpBanRequest {
                                    ip: app.admin_ip_ban_ip.trim().to_string(),
                                    reason: app.admin_ip_ban_reason.trim().to_string(),
                                    permanent: app.admin_ip_ban_perm,
                                    duration_hours: hours,
                                });
                                app.admin_ip_ban_ip.clear();
                                app.admin_ip_ban_reason.clear();
                            }
                        });
                    });
                    ui.add_space(8.0);

                    if let Some(bans) = app.admin_ip_bans.clone() {
                        ui.label(
                            egui::RichText::new(format!("Bans activos ({})", bans.active.len()))
                                .strong(),
                        );
                        ui.add_space(4.0);
                        for b in &bans.active {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&b.ip).strong());
                                    badge_pill(
                                        ui,
                                        &b.expires_at,
                                        p.danger.gamma_multiply(0.18),
                                        p.danger,
                                    );
                                    ui.label(egui::RichText::new(&b.reason).small());
                                    if let Some(by) = &b.banned_by_username {
                                        ui.label(
                                            egui::RichText::new(format!("por @{by}"))
                                                .small()
                                                .color(p.text_muted),
                                        );
                                    }
                                    if ui.button("Revocar").clicked() {
                                        revoke_ip_ban = Some(b.id);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }

                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("Historial ({})", bans.history.len()))
                                .strong(),
                        );
                        ui.add_space(4.0);
                        for b in &bans.history {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&b.ip).strong().small());
                                    ui.label(egui::RichText::new(&b.reason).small());
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} · {}{}",
                                            b.created_at,
                                            b.expires_at,
                                            if b.revoked { " · Revocado" } else { "" }
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                });
                            });
                            ui.add_space(3.0);
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Bans de IP sin cargar.")
                                    .small()
                                    .color(p.text_muted),
                            );
                            if ui.button("Cargar bans IP").clicked() {
                                fetch_ip_bans = true;
                            }
                        });
                    }
                }
                4 => {
                    if app.admin_config.is_some() {
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("Configuración global del sistema").strong());
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Nombre del estudio").small());
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_cfg_studio)
                                    .desired_width(ui.available_width().min(320.0)),
                            );
                            ui.label(egui::RichText::new("Fecha de fundación").small());
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_cfg_date)
                                    .desired_width(ui.available_width().min(220.0)),
                            );
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Rol por defecto al registrarse:").small());
                                egui::ComboBox::from_id_salt("cfg_default_role_cb")
                                    .selected_text(&app.admin_cfg_default_role)
                                    .show_ui(ui, |ui| {
                                        for r in ["usuario", "staff", "admin"] {
                                            ui.selectable_value(
                                                &mut app.admin_cfg_default_role,
                                                r.to_string(),
                                                r,
                                            );
                                        }
                                    });
                            });
                            ui.checkbox(
                                &mut app.admin_cfg_register,
                                "Permitir registro público de cuentas",
                            );
                            ui.checkbox(
                                &mut app.admin_cfg_maintenance,
                                "Modo mantenimiento activo",
                            );
                            ui.add_space(6.0);
                            if primary_button(ui, p, "💾 Guardar configuración").clicked() {
                                save_config = Some(SystemConfigDto {
                                    studio_name: app.admin_cfg_studio.trim().to_string(),
                                    foundation_date: app.admin_cfg_date.trim().to_string(),
                                    allow_public_register: app.admin_cfg_register,
                                    default_role: app.admin_cfg_default_role.clone(),
                                    maintenance_mode: app.admin_cfg_maintenance,
                                });
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Configuración del sistema sin cargar.")
                                    .small()
                                    .color(p.text_muted),
                            );
                            if ui.button("Cargar configuración").clicked() {
                                fetch_config = true;
                            }
                        });
                    }
                }
                5 => {
                    if let Some(roles_payload) = roles_snapshot.clone() {
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("Crear nuevo rol").strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_role_new_name)
                                        .desired_width(160.0)
                                        .hint_text("Nombre del rol"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_role_new_color)
                                        .desired_width(95.0)
                                        .hint_text("#8b5cf6"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_role_new_pos)
                                        .desired_width(65.0)
                                        .hint_text("Posición"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_role_new_desc)
                                        .desired_width(210.0)
                                        .hint_text("Descripción (opcional)"),
                                );
                                if primary_button(ui, p, "➕ Crear rol").clicked()
                                    && !app.admin_role_new_name.trim().is_empty()
                                {
                                    let pos = app
                                        .admin_role_new_pos
                                        .trim()
                                        .parse::<i64>()
                                        .unwrap_or(150);
                                    let color = {
                                        let c = app.admin_role_new_color.trim().to_string();
                                        if c.is_empty() { None } else { Some(c) }
                                    };
                                    let description = {
                                        let d = app.admin_role_new_desc.trim().to_string();
                                        if d.is_empty() { None } else { Some(d) }
                                    };
                                    create_role = Some(SaveRoleRequest {
                                        name: app.admin_role_new_name.trim().to_string(),
                                        color,
                                        position: pos,
                                        description,
                                    });
                                    app.admin_role_new_name.clear();
                                    app.admin_role_new_desc.clear();
                                }
                            });
                        });
                        ui.add_space(8.0);

                        if let Some((rid, rname, ucount)) = app.admin_role_delete_target.clone() {
                            elevated_frame(p).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "¿Eliminar rol \"{rname}\" ({ucount} usuarios)?"
                                    ))
                                    .strong()
                                    .color(p.danger),
                                );
                                ui.horizontal_wrapped(|ui| {
                                    if ucount > 0 {
                                        ui.label("Reasignar usuarios a:");
                                        let sel_txt = app
                                            .admin_role_reassign_id
                                            .and_then(|id| {
                                                roles_payload
                                                    .roles
                                                    .iter()
                                                    .find(|r| r.id == id)
                                                    .map(|r| r.name.clone())
                                            })
                                            .unwrap_or_else(|| "Elegir rol…".to_string());
                                        let mut chosen = app.admin_role_reassign_id.unwrap_or(0);
                                        egui::ComboBox::from_id_salt("role_reassign_cb")
                                            .selected_text(sel_txt)
                                            .show_ui(ui, |ui| {
                                                for r in
                                                    roles_payload.roles.iter().filter(|r| r.id != rid)
                                                {
                                                    if ui
                                                        .selectable_value(&mut chosen, r.id, &r.name)
                                                        .clicked()
                                                    {
                                                        app.admin_role_reassign_id = Some(r.id);
                                                    }
                                                }
                                            });
                                    }
                                    let can_confirm =
                                        ucount == 0 || app.admin_role_reassign_id.is_some();
                                    if ui
                                        .add_enabled(
                                            can_confirm,
                                            egui::Button::new(
                                                egui::RichText::new("Confirmar eliminar")
                                                    .color(p.danger),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        delete_role = Some((rid, app.admin_role_reassign_id));
                                        app.admin_role_delete_target = None;
                                        app.admin_role_reassign_id = None;
                                    }
                                    if ui.button("Cancelar").clicked() {
                                        app.admin_role_delete_target = None;
                                        app.admin_role_reassign_id = None;
                                    }
                                });
                            });
                            ui.add_space(8.0);
                        }

                        for r in &roles_payload.roles {
                            let is_sel = app.admin_role_selected_id == Some(r.id);
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let col = parse_hex_color(&r.resolved_color, p.accent_secondary);
                                    badge_pill(ui, &r.name, col.gamma_multiply(0.20), col);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "@{} · pos {} · {} usuarios",
                                            r.role_key, r.position, r.user_count
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                    if r.is_system {
                                        badge_pill(
                                            ui,
                                            "Sistema",
                                            p.bg_surface,
                                            p.text_muted,
                                        );
                                    }
                                    if r.is_protected {
                                        badge_pill(
                                            ui,
                                            "Protegido",
                                            p.accent_primary.gamma_multiply(0.18),
                                            p.accent_primary,
                                        );
                                    }
                                    if ui
                                        .selectable_label(is_sel, "⚙ Permisos / Secciones")
                                        .clicked()
                                    {
                                        app.admin_role_selected_id = Some(r.id);
                                        app.admin_role_perms_draft = r.perms.clone();
                                        app.admin_role_webs_draft = r.web_ids.clone();
                                    }
                                    if !r.is_protected && ui.button("✏ Editar").clicked() {
                                        app.admin_role_editing_id = Some(r.id);
                                        app.admin_role_edit_name = r.name.clone();
                                        app.admin_role_edit_color = r.color.clone();
                                        app.admin_role_edit_pos = r.position.to_string();
                                        app.admin_role_edit_desc = r.description.clone();
                                    }
                                    if !r.is_system
                                        && ui
                                            .button(egui::RichText::new("🗑").color(p.danger))
                                            .on_hover_text("Eliminar rol")
                                            .clicked()
                                    {
                                        app.admin_role_reassign_id = if r.user_count > 0 {
                                            roles_payload
                                                .roles
                                                .iter()
                                                .find(|o| o.id != r.id && o.role_key == "usuario")
                                                .or_else(|| {
                                                    roles_payload.roles.iter().find(|o| o.id != r.id)
                                                })
                                                .map(|o| o.id)
                                        } else {
                                            None
                                        };
                                        app.admin_role_delete_target =
                                            Some((r.id, r.name.clone(), r.user_count));
                                    }
                                });
                                if !r.description.trim().is_empty() {
                                    ui.label(
                                        egui::RichText::new(&r.description)
                                            .small()
                                            .color(p.text_muted),
                                    );
                                }
                                if app.admin_role_editing_id == Some(r.id) {
                                    ui.separator();
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.admin_role_edit_name)
                                                .desired_width(150.0)
                                                .hint_text("Nombre"),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut app.admin_role_edit_color,
                                            )
                                            .desired_width(95.0)
                                            .hint_text("#color"),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.admin_role_edit_pos)
                                                .desired_width(60.0)
                                                .hint_text("Pos"),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.admin_role_edit_desc)
                                                .desired_width(180.0)
                                                .hint_text("Descripción"),
                                        );
                                        if primary_button(ui, p, "💾 Guardar").clicked()
                                            && !app.admin_role_edit_name.trim().is_empty()
                                        {
                                            let pos = app
                                                .admin_role_edit_pos
                                                .trim()
                                                .parse::<i64>()
                                                .unwrap_or(r.position);
                                            let color = {
                                                let c = app.admin_role_edit_color.trim().to_string();
                                                if c.is_empty() { None } else { Some(c) }
                                            };
                                            let description = {
                                                let d = app.admin_role_edit_desc.trim().to_string();
                                                if d.is_empty() { None } else { Some(d) }
                                            };
                                            update_role = Some((
                                                r.id,
                                                SaveRoleRequest {
                                                    name: app.admin_role_edit_name.trim().to_string(),
                                                    color,
                                                    position: pos,
                                                    description,
                                                },
                                            ));
                                            app.admin_role_editing_id = None;
                                        }
                                        if ui.button("Cancelar").clicked() {
                                            app.admin_role_editing_id = None;
                                        }
                                    });
                                }
                            });
                            ui.add_space(4.0);
                        }

                        if let Some(sel_id) = app.admin_role_selected_id {
                            if let Some(sel_role) = roles_payload.roles.iter().find(|r| r.id == sel_id)
                            {
                                ui.add_space(8.0);
                                elevated_frame(p).show(ui, |ui| {
                                    ui.heading(format!(
                                        "Permisos y secciones: {}",
                                        sel_role.name
                                    ));
                                    if sel_role.is_protected {
                                        ui.label(
                                            egui::RichText::new(
                                                "Rol protegido: tiene acceso total y no puede restringirse.",
                                            )
                                            .small()
                                            .color(p.text_muted),
                                        );
                                    }

                                    let mut categories: Vec<String> = Vec::new();
                                    for perm in &roles_payload.all_perms {
                                        if !categories.contains(&perm.category) {
                                            categories.push(perm.category.clone());
                                        }
                                    }
                                    for cat in &categories {
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(cat)
                                                .strong()
                                                .small()
                                                .color(p.accent_secondary),
                                        );
                                        ui.horizontal_wrapped(|ui| {
                                            for perm in roles_payload
                                                .all_perms
                                                .iter()
                                                .filter(|pm| &pm.category == cat)
                                            {
                                                let mut has = app
                                                    .admin_role_perms_draft
                                                    .iter()
                                                    .any(|k| k == &perm.key);
                                                let resp = ui.add_enabled(
                                                    !sel_role.is_protected,
                                                    egui::Checkbox::new(&mut has, &perm.label),
                                                );
                                                if resp.changed() {
                                                    if has {
                                                        if !app
                                                            .admin_role_perms_draft
                                                            .iter()
                                                            .any(|k| k == &perm.key)
                                                        {
                                                            app.admin_role_perms_draft
                                                                .push(perm.key.clone());
                                                        }
                                                    } else {
                                                        app.admin_role_perms_draft
                                                            .retain(|k| k != &perm.key);
                                                    }
                                                }
                                            }
                                        });
                                    }

                                    if !sel_role.is_protected {
                                        ui.add_space(6.0);
                                        if primary_button(ui, p, "💾 Guardar permisos").clicked() {
                                            save_role_perms =
                                                Some((sel_id, app.admin_role_perms_draft.clone()));
                                        }
                                    }

                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new("Secciones / Webs permitidas")
                                            .strong()
                                            .small(),
                                    );
                                    ui.horizontal_wrapped(|ui| {
                                        for w in &roles_payload.all_webs {
                                            let mut has =
                                                app.admin_role_webs_draft.contains(&w.id);
                                            let lbl = format!("{} {}", w.icon, w.name);
                                            let resp = ui.add_enabled(
                                                !sel_role.is_protected,
                                                egui::Checkbox::new(&mut has, lbl),
                                            );
                                            if resp.changed() {
                                                if has {
                                                    if !app.admin_role_webs_draft.contains(&w.id) {
                                                        app.admin_role_webs_draft.push(w.id);
                                                    }
                                                } else {
                                                    app.admin_role_webs_draft
                                                        .retain(|id| *id != w.id);
                                                }
                                            }
                                        }
                                    });
                                    if !sel_role.is_protected {
                                        ui.add_space(6.0);
                                        if primary_button(ui, p, "💾 Guardar secciones").clicked() {
                                            save_role_webs =
                                                Some((sel_id, app.admin_role_webs_draft.clone()));
                                        }
                                    }
                                });
                            }
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Roles sin cargar.")
                                    .small()
                                    .color(p.text_muted),
                            );
                            if ui.button("Cargar roles").clicked() {
                                fetch_roles = true;
                            }
                        });
                    }
                }
                6 => {
                    if let Some(titles_payload) = app.admin_titles.clone() {
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(if app.admin_title_editing_id.is_some() {
                                    "Editar título"
                                } else {
                                    "Crear nuevo título"
                                })
                                .strong(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_title_name)
                                        .desired_width(160.0)
                                        .hint_text("Nombre del título"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_title_color)
                                        .desired_width(95.0)
                                        .hint_text("#e0a308"),
                                );
                                egui::ComboBox::from_id_salt("title_effect_cb")
                                    .selected_text(&app.admin_title_effect)
                                    .show_ui(ui, |ui| {
                                        for eff in ["none", "glow", "shimmer", "gradient"] {
                                            ui.selectable_value(
                                                &mut app.admin_title_effect,
                                                eff.to_string(),
                                                eff,
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_title_desc)
                                        .desired_width(200.0)
                                        .hint_text("Descripción (opcional)"),
                                );
                                let submit_lbl = if app.admin_title_editing_id.is_some() {
                                    "💾 Guardar título"
                                } else {
                                    "➕ Crear título"
                                };
                                if primary_button(ui, p, submit_lbl).clicked()
                                    && !app.admin_title_name.trim().is_empty()
                                {
                                    let color = {
                                        let c = app.admin_title_color.trim().to_string();
                                        if c.is_empty() { None } else { Some(c) }
                                    };
                                    let description = {
                                        let d = app.admin_title_desc.trim().to_string();
                                        if d.is_empty() { None } else { Some(d) }
                                    };
                                    let req = SaveTitleRequest {
                                        name: app.admin_title_name.trim().to_string(),
                                        color,
                                        effect: app.admin_title_effect.clone(),
                                        description,
                                    };
                                    if let Some(tid) = app.admin_title_editing_id.take() {
                                        update_title = Some((tid, req));
                                    } else {
                                        create_title = Some(req);
                                    }
                                    app.admin_title_name.clear();
                                    app.admin_title_desc.clear();
                                }
                                if app.admin_title_editing_id.is_some() && ui.button("Cancelar").clicked()
                                {
                                    app.admin_title_editing_id = None;
                                    app.admin_title_name.clear();
                                    app.admin_title_desc.clear();
                                }
                            });
                        });
                        ui.add_space(8.0);

                        for t in &titles_payload.titles {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let col =
                                        parse_hex_color(&t.resolved_color, p.accent_primary);
                                    badge_pill(ui, &t.name, col.gamma_multiply(0.20), col);
                                    badge_pill(
                                        ui,
                                        &t.effect,
                                        p.accent_secondary.gamma_multiply(0.16),
                                        p.accent_secondary,
                                    );
                                    if ui.button("✏ Editar").clicked() {
                                        app.admin_title_editing_id = Some(t.id);
                                        app.admin_title_name = t.name.clone();
                                        app.admin_title_color = t.color.clone();
                                        app.admin_title_effect = t.effect.clone();
                                        app.admin_title_desc = t.description.clone();
                                    }
                                    if ui
                                        .button(egui::RichText::new("🗑 Eliminar").color(p.danger))
                                        .clicked()
                                    {
                                        delete_title = Some(t.id);
                                    }
                                });
                                if !t.description.trim().is_empty() {
                                    ui.label(
                                        egui::RichText::new(&t.description)
                                            .small()
                                            .color(p.text_muted),
                                    );
                                }

                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Portadores ({}):",
                                            t.holders.len()
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                    for h in &t.holders {
                                        ui.label(
                                            egui::RichText::new(format!("@{}", h.username))
                                                .small()
                                                .strong(),
                                        );
                                        if ui
                                            .small_button(egui::RichText::new("✕").color(p.danger))
                                            .on_hover_text("Revocar título")
                                            .clicked()
                                        {
                                            revoke_title = Some((t.id, h.user_id));
                                        }
                                    }
                                });

                                if !titles_payload.users.is_empty() {
                                    let entry = app
                                        .admin_title_assign_user
                                        .entry(t.id)
                                        .or_insert(titles_payload.users[0].id);
                                    let sel_label = titles_payload
                                        .users
                                        .iter()
                                        .find(|u| u.id == *entry)
                                        .map(|u| format!("{} (@{})", u.display, u.username))
                                        .unwrap_or_else(|| "Elegir usuario".to_string());
                                    ui.horizontal_wrapped(|ui| {
                                        egui::ComboBox::from_id_salt(("title_assign_cb", t.id))
                                            .selected_text(sel_label)
                                            .show_ui(ui, |ui| {
                                                for u in &titles_payload.users {
                                                    ui.selectable_value(
                                                        entry,
                                                        u.id,
                                                        format!("{} (@{})", u.display, u.username),
                                                    );
                                                }
                                            });
                                        if ui.button("➕ Asignar").clicked() {
                                            assign_title = Some((t.id, *entry));
                                        }
                                    });
                                }
                            });
                            ui.add_space(5.0);
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Títulos sin cargar.")
                                    .small()
                                    .color(p.text_muted),
                            );
                            if ui.button("Cargar títulos").clicked() {
                                fetch_titles = true;
                            }
                        });
                    }
                }
                7 => {
                    if let Some(apis) = app.admin_apis.clone() {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⚙ Rotación, Failover y Límites de Aris").strong());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🔓 Liberar cooldowns").clicked() {
                                        reset_api_cooldowns = true;
                                    }
                                });
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut app.admin_apis_cfg_rot_enabled, "Rotación activa");
                                ui.checkbox(&mut app.admin_apis_cfg_failover, "Auto failover");
                                ui.label("Estrategia:");
                                egui::ComboBox::from_id_salt("aris_rot_strat_cb")
                                    .selected_text(&app.admin_apis_cfg_rot_strat)
                                    .show_ui(ui, |ui| {
                                        for s in ["round_robin", "least_used", "provider_order"] {
                                            ui.selectable_value(
                                                &mut app.admin_apis_cfg_rot_strat,
                                                s.to_string(),
                                                s,
                                            );
                                        }
                                    });
                                ui.label("Cooldown (min):");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_cooldown)
                                        .desired_width(55.0),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Lím. hora:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_hourly)
                                        .desired_width(55.0),
                                );
                                ui.label("Lím. día:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_daily)
                                        .desired_width(55.0),
                                );
                                ui.label("Créditos/día:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_credits)
                                        .desired_width(55.0),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ollama URL:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_ollama)
                                        .desired_width(230.0),
                                );
                                ui.label("AethraMail URL:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_cfg_mail_url)
                                        .desired_width(230.0),
                                );
                            });
                            ui.label(egui::RichText::new("System prompt de Aris:").small().color(p.text_muted));
                            ui.add(
                                egui::TextEdit::multiline(&mut app.admin_apis_cfg_sys_prompt)
                                    .desired_width(ui.available_width().min(680.0))
                                    .desired_rows(3),
                            );
                            if primary_button(ui, p, "💾 Guardar config Aris").clicked() {
                                save_apis_cfg = Some(AdminApisConfigDto {
                                    rotation_enabled: app.admin_apis_cfg_rot_enabled,
                                    rotation_strategy: app.admin_apis_cfg_rot_strat.clone(),
                                    cooldown_minutes: app
                                        .admin_apis_cfg_cooldown
                                        .trim()
                                        .parse::<i64>()
                                        .unwrap_or(60)
                                        .clamp(1, 1440),
                                    auto_failover: app.admin_apis_cfg_failover,
                                    ollama_url: app.admin_apis_cfg_ollama.trim().to_string(),
                                    hourly_limit: app
                                        .admin_apis_cfg_hourly
                                        .trim()
                                        .parse::<i64>()
                                        .unwrap_or(30)
                                        .max(1),
                                    daily_limit: app
                                        .admin_apis_cfg_daily
                                        .trim()
                                        .parse::<i64>()
                                        .unwrap_or(150)
                                        .max(1),
                                    daily_credits_limit: app
                                        .admin_apis_cfg_credits
                                        .trim()
                                        .parse::<i64>()
                                        .unwrap_or(120)
                                        .max(10),
                                    aethramail_url: app.admin_apis_cfg_mail_url.trim().to_string(),
                                    system_prompt: app.admin_apis_cfg_sys_prompt.clone(),
                                });
                            }
                        });
                        ui.add_space(8.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("🔑 Añadir API Key(s)").strong());
                            ui.horizontal_wrapped(|ui| {
                                let prov_label = apis
                                    .providers
                                    .iter()
                                    .find(|pr| pr.id == app.admin_apis_key_provider_id)
                                    .map(|pr| pr.name.clone())
                                    .unwrap_or_else(|| "Proveedor".into());
                                egui::ComboBox::from_id_salt("add_key_prov_cb")
                                    .selected_text(prov_label)
                                    .show_ui(ui, |ui| {
                                        for pr in &apis.providers {
                                            ui.selectable_value(
                                                &mut app.admin_apis_key_provider_id,
                                                pr.id,
                                                &pr.name,
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_key_label)
                                        .desired_width(130.0)
                                        .hint_text("Etiqueta"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_key_cap)
                                        .desired_width(80.0)
                                        .hint_text("Tope/día"),
                                );
                            });
                            ui.add(
                                egui::TextEdit::multiline(&mut app.admin_apis_key_text)
                                    .desired_width(ui.available_width().min(560.0))
                                    .desired_rows(2)
                                    .hint_text("Una o varias API keys (una por línea)…"),
                            );
                            if primary_button(ui, p, "➕ Agregar key(s)").clicked()
                                && !app.admin_apis_key_text.trim().is_empty()
                            {
                                let cap = app
                                    .admin_apis_key_cap
                                    .trim()
                                    .parse::<i64>()
                                    .ok()
                                    .filter(|&c| c > 0);
                                add_api_key_req = Some(AdminAddApiKeyRequest {
                                    provider_id: app.admin_apis_key_provider_id,
                                    api_key: app.admin_apis_key_text.clone(),
                                    label: if app.admin_apis_key_label.trim().is_empty() {
                                        None
                                    } else {
                                        Some(app.admin_apis_key_label.trim().to_string())
                                    },
                                    daily_cap: cap,
                                });
                                app.admin_apis_key_text.clear();
                                app.admin_apis_key_label.clear();
                            }
                        });
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("Keys registradas ({})", apis.keys.len())).strong());
                        for k in &apis.keys {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&k.label).strong());
                                    ui.label(egui::RichText::new(&k.masked_key).monospace().small().color(p.text_muted));
                                    badge_pill(
                                        ui,
                                        &k.provider_name,
                                        p.accent_secondary.gamma_multiply(0.16),
                                        p.accent_secondary,
                                    );
                                    badge_pill(
                                        ui,
                                        if k.is_active { "Activa" } else { "Pausada" },
                                        if k.is_active { p.success.gamma_multiply(0.18) } else { p.danger.gamma_multiply(0.18) },
                                        if k.is_active { p.success } else { p.danger },
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("Hoy: {} · Total: {}", k.daily_uses, k.total_uses))
                                            .small()
                                            .color(p.text_muted),
                                    );
                                    let cap_entry = app.admin_apis_key_caps_draft.entry(k.id).or_default();
                                    ui.add(
                                        egui::TextEdit::singleline(cap_entry)
                                            .desired_width(60.0)
                                            .hint_text("∞"),
                                    );
                                    if ui.small_button("Tope").clicked() {
                                        let c = cap_entry.trim().parse::<i64>().ok().filter(|&v| v > 0);
                                        cap_api_key = Some((k.id, c));
                                    }
                                    if ui.button("🧪 Test").clicked() {
                                        test_api_key = Some(k.id);
                                    }
                                    if ui.button(if k.is_active { "Pausar" } else { "Activar" }).clicked() {
                                        toggle_api_key = Some((k.id, !k.is_active));
                                    }
                                    if ui.button(egui::RichText::new("🗑").color(p.danger)).clicked() {
                                        delete_api_key = Some(k.id);
                                    }
                                });
                                if let Some((ok, msg)) = app.admin_apis_key_test_msg.get(&k.id) {
                                    ui.label(
                                        egui::RichText::new(msg)
                                            .small()
                                            .color(if *ok { p.success } else { p.danger }),
                                    );
                                } else if let Some(err) = &k.last_error {
                                    if !err.is_empty() {
                                        ui.label(egui::RichText::new(err).small().color(p.danger));
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }
                        ui.add_space(8.0);
                        elevated_frame(p).show(ui, |ui| {
                            let editing = app.admin_apis_prov_editing_id.is_some();
                            ui.label(
                                egui::RichText::new(if editing {
                                    "Editar proveedor"
                                } else {
                                    "Nuevo proveedor de IA"
                                })
                                .strong(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_prov_name)
                                        .desired_width(140.0)
                                        .hint_text("Nombre"),
                                );
                                egui::ComboBox::from_id_salt("prov_type_cb")
                                    .selected_text(&app.admin_apis_prov_type)
                                    .show_ui(ui, |ui| {
                                        for pt in ["openai_compat", "gemini", "anthropic", "groq", "ollama"] {
                                            ui.selectable_value(
                                                &mut app.admin_apis_prov_type,
                                                pt.to_string(),
                                                pt,
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_prov_url)
                                        .desired_width(220.0)
                                        .hint_text("Base URL"),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_apis_prov_model)
                                        .desired_width(140.0)
                                        .hint_text("Modelo por defecto"),
                                );
                                let btn_lbl = if editing { "💾 Guardar" } else { "➕ Añadir proveedor" };
                                if primary_button(ui, p, btn_lbl).clicked()
                                    && !app.admin_apis_prov_name.trim().is_empty()
                                    && !app.admin_apis_prov_url.trim().is_empty()
                                {
                                    let req = AdminSaveProviderRequest {
                                        name: app.admin_apis_prov_name.trim().to_string(),
                                        provider_type: app.admin_apis_prov_type.clone(),
                                        base_url: app.admin_apis_prov_url.trim().to_string(),
                                        default_model: if app.admin_apis_prov_model.trim().is_empty() {
                                            None
                                        } else {
                                            Some(app.admin_apis_prov_model.trim().to_string())
                                        },
                                    };
                                    if let Some(pid) = app.admin_apis_prov_editing_id.take() {
                                        update_provider_req = Some((pid, req));
                                    } else {
                                        add_provider_req = Some(req);
                                    }
                                    app.admin_apis_prov_name.clear();
                                    app.admin_apis_prov_url.clear();
                                    app.admin_apis_prov_model.clear();
                                }
                                if editing && ui.button("Cancelar").clicked() {
                                    app.admin_apis_prov_editing_id = None;
                                    app.admin_apis_prov_name.clear();
                                    app.admin_apis_prov_url.clear();
                                    app.admin_apis_prov_model.clear();
                                }
                            });
                        });
                        ui.add_space(6.0);
                        for pr in &apis.providers {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(format!("#{} {}", pr.priority, pr.name)).strong());
                                    badge_pill(
                                        ui,
                                        &pr.provider_type,
                                        p.accent_primary.gamma_multiply(0.14),
                                        p.accent_primary,
                                    );
                                    ui.label(egui::RichText::new(&pr.default_model).small().color(p.text_muted));
                                    if ui.small_button("⬆").clicked() {
                                        reorder_provider = Some((pr.id, -1));
                                    }
                                    if ui.small_button("⬇").clicked() {
                                        reorder_provider = Some((pr.id, 1));
                                    }
                                    if ui.button("✏").clicked() {
                                        app.admin_apis_prov_editing_id = Some(pr.id);
                                        app.admin_apis_prov_name = pr.name.clone();
                                        app.admin_apis_prov_type = pr.provider_type.clone();
                                        app.admin_apis_prov_url = pr.base_url.clone();
                                        app.admin_apis_prov_model = pr.default_model.clone();
                                    }
                                    if ui.button(if pr.is_active { "Pausar" } else { "Activar" }).clicked() {
                                        toggle_provider = Some((pr.id, !pr.is_active));
                                    }
                                    if !pr.is_preset
                                        && ui.button(egui::RichText::new("🗑").color(p.danger)).clicked()
                                    {
                                        delete_provider = Some(pr.id);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Bitácora de rotación ({})", apis.logs.len())).strong());
                            if ui.button("🧹 Vaciar bitácora").clicked() {
                                clear_rot_logs = true;
                            }
                        });
                        for l in &apis.logs {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&l.created_at).small().color(p.text_muted));
                                    badge_pill(
                                        ui,
                                        &l.status,
                                        if l.status == "ok" { p.success.gamma_multiply(0.18) } else { p.danger.gamma_multiply(0.18) },
                                        if l.status == "ok" { p.success } else { p.danger },
                                    );
                                    ui.label(egui::RichText::new(format!("{} / {} ({} ms)", l.provider_name, l.model, l.latency_ms)).small());
                                    if !l.detail.is_empty() {
                                        ui.label(egui::RichText::new(&l.detail).small().color(p.text_muted));
                                    }
                                });
                            });
                            ui.add_space(3.0);
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Configuración de APIs sin cargar.").small().color(p.text_muted));
                            if ui.button("Cargar APIs").clicked() {
                                fetch_apis = true;
                            }
                        });
                    }
                }
                8 => {
                    elevated_frame(p).show(ui, |ui| {
                        ui.label(egui::RichText::new("📄 Subir documento (PDF, DOCX, PPTX)").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_doc_up_title)
                                    .desired_width(220.0)
                                    .hint_text("Título del documento"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_doc_up_desc)
                                    .desired_width(260.0)
                                    .hint_text("Descripción (opcional)"),
                            );
                        });
                        if let Some(fname) = &app.admin_doc_up_dropped_name.clone() {
                            ui.horizontal(|ui| {
                                badge_pill(
                                    ui,
                                    &format!("📎 {fname}"),
                                    p.success.gamma_multiply(0.18),
                                    p.success,
                                );
                                if ui.small_button("Quitar").clicked() {
                                    app.admin_doc_up_dropped_name = None;
                                    app.admin_doc_up_dropped_bytes = None;
                                }
                            });
                        } else {
                            #[cfg(not(target_arch = "wasm32"))]
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_doc_up_path)
                                    .desired_width(ui.available_width().min(480.0))
                                    .hint_text("Arrastra un archivo aquí o escribe su ruta local…"),
                            );
                            #[cfg(target_arch = "wasm32")]
                            ui.label(
                                egui::RichText::new("Arrastra y suelta el PDF/DOCX/PPTX sobre la ventana.")
                                    .small()
                                    .color(p.text_muted),
                            );
                        }
                        if primary_button(ui, p, "📤 Subir documento").clicked()
                            && !app.admin_doc_up_title.trim().is_empty()
                        {
                            let desc = if app.admin_doc_up_desc.trim().is_empty() {
                                None
                            } else {
                                Some(app.admin_doc_up_desc.trim().to_string())
                            };
                            if let (Some(fname), Some(bytes)) = (
                                app.admin_doc_up_dropped_name.take(),
                                app.admin_doc_up_dropped_bytes.take(),
                            ) {
                                doc_upload_bytes = Some((
                                    app.admin_doc_up_title.trim().to_string(),
                                    desc,
                                    fname,
                                    bytes,
                                ));
                                app.admin_doc_up_title.clear();
                                app.admin_doc_up_desc.clear();
                            } else if !app.admin_doc_up_path.trim().is_empty() {
                                doc_upload_path = Some((
                                    app.admin_doc_up_title.trim().to_string(),
                                    desc,
                                    app.admin_doc_up_path.clone(),
                                ));
                                app.admin_doc_up_title.clear();
                                app.admin_doc_up_desc.clear();
                                app.admin_doc_up_path.clear();
                            }
                        }
                    });
                    if let Some(edit_id) = app.admin_doc_edit_id {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new(format!("Editar documento #{edit_id}")).strong());
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_doc_edit_title)
                                        .desired_width(220.0),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.admin_doc_edit_desc)
                                        .desired_width(260.0),
                                );
                                if primary_button(ui, p, "Guardar").clicked()
                                    && !app.admin_doc_edit_title.trim().is_empty()
                                {
                                    doc_update = Some((
                                        edit_id,
                                        AdminUpdateDocRequest {
                                            title: app.admin_doc_edit_title.trim().to_string(),
                                            description: if app.admin_doc_edit_desc.trim().is_empty() {
                                                None
                                            } else {
                                                Some(app.admin_doc_edit_desc.trim().to_string())
                                            },
                                        },
                                    ));
                                    app.admin_doc_edit_id = None;
                                }
                                if ui.button("Cancelar").clicked() {
                                    app.admin_doc_edit_id = None;
                                }
                            });
                        });
                    }
                    ui.add_space(8.0);
                    if let Some(docs) = &app.admin_docs {
                        for d in docs {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&d.icon).size(18.0));
                                    ui.label(egui::RichText::new(&d.title).strong());
                                    ui.label(
                                        egui::RichText::new(format!("{} · {} · {} descargas", d.file_name, d.size, d.downloaded))
                                            .small()
                                            .color(p.text_muted),
                                    );
                                    if ui.button("✏ Editar").clicked() {
                                        app.admin_doc_edit_id = Some(d.id);
                                        app.admin_doc_edit_title = d.title.clone();
                                        app.admin_doc_edit_desc = d.description.clone();
                                    }
                                    if d.file_type != "pdf" && ui.button("↻ Preview").clicked() {
                                        doc_regenerate = Some(d.id);
                                    }
                                    if ui.button(egui::RichText::new("🗑").color(p.danger)).clicked() {
                                        doc_delete = Some(d.id);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    } else if ui.button("Cargar documentos").clicked() {
                        fetch_docs = true;
                    }
                }
                9 => {
                    elevated_frame(p).show(ui, |ui| {
                        ui.label(egui::RichText::new("⬇ Publicar ejecutable o paquete").strong());
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_id_salt("dl_plat_cb")
                                .selected_text(&app.admin_dl_platform)
                                .show_ui(ui, |ui| {
                                    for pl in ["windows", "linux", "android", "macos", "ios"] {
                                        ui.selectable_value(
                                            &mut app.admin_dl_platform,
                                            pl.to_string(),
                                            pl,
                                        );
                                    }
                                });
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_dl_slug)
                                    .desired_width(120.0)
                                    .hint_text("slug (aethravox)"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_dl_name)
                                    .desired_width(160.0)
                                    .hint_text("Nombre app"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_dl_version)
                                    .desired_width(80.0)
                                    .hint_text("1.0.0"),
                            );
                        });
                        ui.add(
                            egui::TextEdit::singleline(&mut app.admin_dl_notes)
                                .desired_width(ui.available_width().min(480.0))
                                .hint_text("Notas de versión (opcional)"),
                        );
                        if let Some(fname) = &app.admin_dl_dropped_name.clone() {
                            ui.horizontal(|ui| {
                                badge_pill(
                                    ui,
                                    &format!("📦 {fname}"),
                                    p.success.gamma_multiply(0.18),
                                    p.success,
                                );
                                if ui.small_button("Quitar").clicked() {
                                    app.admin_dl_dropped_name = None;
                                    app.admin_dl_dropped_bytes = None;
                                }
                            });
                        } else {
                            #[cfg(not(target_arch = "wasm32"))]
                            ui.add(
                                egui::TextEdit::singleline(&mut app.admin_dl_path)
                                    .desired_width(ui.available_width().min(480.0))
                                    .hint_text("Arrastra un binario (.exe, .apk, .zip, .AppImage…) o escribe su ruta…"),
                            );
                            #[cfg(target_arch = "wasm32")]
                            ui.label(
                                egui::RichText::new("Arrastra y suelta el binario sobre la ventana.")
                                    .small()
                                    .color(p.text_muted),
                            );
                        }
                        if primary_button(ui, p, "📤 Publicar descarga").clicked()
                            && !app.admin_dl_slug.trim().is_empty()
                            && !app.admin_dl_name.trim().is_empty()
                        {
                            let notes = if app.admin_dl_notes.trim().is_empty() {
                                None
                            } else {
                                Some(app.admin_dl_notes.trim().to_string())
                            };
                            if let (Some(fname), Some(bytes)) = (
                                app.admin_dl_dropped_name.take(),
                                app.admin_dl_dropped_bytes.take(),
                            ) {
                                dl_upload_bytes = Some((
                                    app.admin_dl_platform.clone(),
                                    app.admin_dl_slug.trim().to_string(),
                                    app.admin_dl_name.trim().to_string(),
                                    app.admin_dl_version.trim().to_string(),
                                    notes,
                                    fname,
                                    bytes,
                                ));
                                app.admin_dl_notes.clear();
                            } else if !app.admin_dl_path.trim().is_empty() {
                                dl_upload_path = Some((
                                    app.admin_dl_platform.clone(),
                                    app.admin_dl_slug.trim().to_string(),
                                    app.admin_dl_name.trim().to_string(),
                                    app.admin_dl_version.trim().to_string(),
                                    notes,
                                    app.admin_dl_path.clone(),
                                ));
                                app.admin_dl_notes.clear();
                                app.admin_dl_path.clear();
                            }
                        }
                    });
                    ui.add_space(8.0);
                    if let Some(rows) = &app.admin_downloads {
                        for r in rows {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    badge_pill(
                                        ui,
                                        &r.platform,
                                        p.accent_primary.gamma_multiply(0.16),
                                        p.accent_primary,
                                    );
                                    ui.label(egui::RichText::new(&r.app_name).strong());
                                    ui.label(
                                        egui::RichText::new(format!("v{} · {} · {:.1} MB · {}", r.version, r.orig_name, r.size_bytes as f64 / (1024.0 * 1024.0), r.created_at))
                                            .small()
                                            .color(p.text_muted),
                                    );
                                    if ui.button(egui::RichText::new("🗑").color(p.danger)).clicked() {
                                        dl_delete = Some(r.id);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    } else if ui.button("Cargar descargas").clicked() {
                        fetch_downloads = true;
                    }
                }
                _ => {}
            });
    } else {
        card_frame(p).show(ui, |ui| {
            ui.label(
                egui::RichText::new(if app.offline_mode {
                    "Panel de administración disponible en modo conectado con permisos admin.access."
                } else {
                    "Cargando estadísticas de administración…"
                })
                .color(p.text_muted),
            );
        });
    }

    if refresh && !app.offline_mode && app.token.is_some() {
        spawn_admin_overview(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
        if app.admin_tab == 3 {
            fetch_ip_bans = true;
        } else if app.admin_tab == 4 {
            fetch_config = true;
        } else if app.admin_tab == 5 {
            fetch_roles = true;
        } else if app.admin_tab == 6 {
            fetch_titles = true;
        } else if app.admin_tab == 7 {
            fetch_apis = true;
        } else if app.admin_tab == 8 {
            fetch_docs = true;
        } else if app.admin_tab == 9 {
            fetch_downloads = true;
        }
    }
    if fetch_ip_bans && !app.offline_mode && app.token.is_some() {
        spawn_admin_ip_bans(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if fetch_config && !app.offline_mode && app.token.is_some() {
        spawn_admin_config_get(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if fetch_roles && !app.offline_mode && app.token.is_some() {
        spawn_admin_roles(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if fetch_titles && !app.offline_mode && app.token.is_some() {
        spawn_admin_titles(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some((uid, role)) = role_change {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_set_role(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
                role,
            );
        }
    }
    if let Some((uid, role_id)) = assign_role_id {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_assign_role_id(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
                role_id,
            );
        }
    }
    if let Some((uid, req)) = submit_ban {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_ban_user(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
                req,
            );
        }
    }
    if let Some((uid, req)) = submit_suspend {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_suspend_user(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
                req,
            );
        }
    }
    if let Some(uid) = submit_delete {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_user(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
            );
        }
    }
    if let Some(req) = create_ip_ban {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_create_ip_ban(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(ban_id) = revoke_ip_ban {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_revoke_ip_ban(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                ban_id,
            );
        }
    }
    if let Some(cfg) = save_config {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_config_save(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cfg,
            );
        }
    }
    if let Some(req) = create_role {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_create_role(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some((role_id, req)) = update_role {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_update_role(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                role_id,
                req,
            );
        }
    }
    if let Some((role_id, reassign_to)) = delete_role {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_role(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                role_id,
                reassign_to,
            );
        }
    }
    if let Some((role_id, perms)) = save_role_perms {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_set_role_perms(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                role_id,
                perms,
            );
        }
    }
    if let Some((role_id, web_ids)) = save_role_webs {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_set_role_web_perms(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                role_id,
                web_ids,
            );
        }
    }
    if let Some(req) = create_title {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_create_title(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some((title_id, req)) = update_title {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_update_title(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title_id,
                req,
            );
        }
    }
    if let Some(title_id) = delete_title {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_title(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title_id,
            );
        }
    }
    if let Some((title_id, user_id)) = assign_title {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_assign_title(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title_id,
                user_id,
            );
        }
    }
    if let Some((title_id, user_id)) = revoke_title {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_revoke_title(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title_id,
                user_id,
            );
        }
    }
    if fetch_apis && !app.offline_mode && app.token.is_some() {
        spawn_admin_apis(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if fetch_docs && !app.offline_mode && app.token.is_some() {
        spawn_admin_docs_list(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if fetch_downloads && !app.offline_mode && app.token.is_some() {
        spawn_admin_downloads_list(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some(uid) = fetch_user_ips {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_user_ips(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                uid,
            );
        }
    }
    if let Some(req) = create_user_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_create_user(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(req) = create_web_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_create_web(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some((wid, html)) = save_web_html {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_update_web_html(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                wid,
                html,
            );
        }
    }
    if let Some((wid, req)) = save_web_tag {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_set_web_tag(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                wid,
                req,
            );
        }
    }
    if let Some(wid) = delete_web_id {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_web(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                wid,
            );
        }
    }
    if let Some(req) = add_api_key_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_add_api_key(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some((kid, active)) = toggle_api_key {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_toggle_api_key(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                kid,
                active,
            );
        }
    }
    if let Some((kid, cap)) = cap_api_key {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_update_api_key_cap(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                kid,
                cap,
            );
        }
    }
    if let Some(kid) = test_api_key {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_test_api_key(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                kid,
            );
        }
    }
    if let Some(kid) = delete_api_key {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_api_key(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                kid,
            );
        }
    }
    if reset_api_cooldowns && !app.offline_mode && app.token.is_some() {
        spawn_admin_reset_api_cooldowns(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some(req) = add_provider_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_add_api_provider(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some((pid, req)) = update_provider_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_update_api_provider(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                pid,
                req,
            );
        }
    }
    if let Some((pid, active)) = toggle_provider {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_toggle_api_provider(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                pid,
                active,
            );
        }
    }
    if let Some((pid, delta)) = reorder_provider {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_reorder_api_provider(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                pid,
                delta,
            );
        }
    }
    if let Some(pid) = delete_provider {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_delete_api_provider(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                pid,
            );
        }
    }
    if let Some(cfg) = save_apis_cfg {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_save_apis_config(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                cfg,
            );
        }
    }
    if clear_rot_logs && !app.offline_mode && app.token.is_some() {
        spawn_admin_clear_rotation_logs(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some((title, desc, fname, bytes)) = doc_upload_bytes {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_doc_upload_bytes(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title,
                desc,
                fname,
                bytes,
            );
        }
    }
    if let Some((title, desc, path)) = doc_upload_path {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_doc_upload_path(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                title,
                desc,
                path,
            );
        }
    }
    if let Some((did, req)) = doc_update {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_doc_update(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                did,
                req,
            );
        }
    }
    if let Some(did) = doc_regenerate {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_doc_regenerate(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                did,
            );
        }
    }
    if let Some(did) = doc_delete {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_doc_delete(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                did,
            );
        }
    }
    if let Some((plat, slug, name, ver, notes, fname, bytes)) = dl_upload_bytes {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_download_create_bytes(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                plat,
                slug,
                name,
                ver,
                notes,
                fname,
                bytes,
            );
        }
    }
    if let Some((plat, slug, name, ver, notes, path)) = dl_upload_path {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_download_create_path(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                plat,
                slug,
                name,
                ver,
                notes,
                path,
            );
        }
    }
    if let Some(dl_id) = dl_delete {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_download_delete(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                dl_id,
            );
        }
    }
    if let Some((path, ok_msg)) = simple_action {
        if !app.offline_mode && app.token.is_some() {
            spawn_admin_simple_action(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                path,
                ok_msg,
            );
        }
    }
}

fn stat_chip(ui: &mut egui::Ui, p: &Palette, label: &str, value: i64) {
    elevated_frame(p).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(value.to_string())
                    .strong()
                    .size(18.0)
                    .color(p.accent_primary),
            );
            ui.label(egui::RichText::new(label).small().color(p.text_muted));
        });
    });
}

