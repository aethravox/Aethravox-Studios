use aethravox_core::AdminBanRequest;

use crate::api::{
    spawn_admin_ban_user, spawn_admin_overview, spawn_admin_set_role, spawn_admin_simple_action,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut role_change: Option<(i64, String)> = None;
    let mut simple_action: Option<(String, &'static str)> = None;
    let mut submit_ban: Option<(i64, AdminBanRequest)> = None;

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
        ui.horizontal(|ui| {
            for (idx, label) in [(0usize, "Usuarios"), (1, "Secciones / Webs"), (2, "Auditoría")] {
                if ui.selectable_label(app.admin_tab == idx, label).clicked() {
                    app.admin_tab = idx;
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

                                if u.status == "banned" || u.status == "suspended" {
                                    if ui.button("Desbanear").clicked() {
                                        simple_action = Some((
                                            format!("/api/v1/admin/users/{}/unban", u.id),
                                            "Usuario desbaneado.",
                                        ));
                                    }
                                } else if ui
                                    .button(egui::RichText::new("Banear").color(p.danger))
                                    .clicked()
                                {
                                    app.admin_ban_target = Some((u.id, u.username.clone()));
                                }

                                if ui.button("Forzar logout").clicked() {
                                    simple_action = Some((
                                        format!("/api/v1/admin/users/{}/force_logout", u.id),
                                        "Sesiones revocadas.",
                                    ));
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
                _ => {
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
