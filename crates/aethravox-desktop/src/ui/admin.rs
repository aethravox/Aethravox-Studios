use aethravox_core::{
    AdminBanRequest, CreateIpBanRequest, SaveRoleRequest, SaveTitleRequest, SuspendUserRequest,
    SystemConfigDto,
};

use crate::api::{
    spawn_admin_assign_role_id, spawn_admin_assign_title, spawn_admin_ban_user,
    spawn_admin_config_get, spawn_admin_config_save, spawn_admin_create_ip_ban,
    spawn_admin_create_role, spawn_admin_create_title, spawn_admin_delete_role,
    spawn_admin_delete_title, spawn_admin_delete_user, spawn_admin_ip_bans, spawn_admin_overview,
    spawn_admin_revoke_ip_ban, spawn_admin_revoke_title, spawn_admin_roles, spawn_admin_set_role,
    spawn_admin_set_role_perms, spawn_admin_set_role_web_perms, spawn_admin_simple_action,
    spawn_admin_suspend_user, spawn_admin_titles, spawn_admin_update_role,
    spawn_admin_update_title,
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

        let roles_snapshot = app.admin_roles.clone();
        egui::ScrollArea::vertical()
            .id_salt("admin_tab_scroll")
            .show(ui, |ui| match app.admin_tab {
                0 => {
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
                _ => {
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

