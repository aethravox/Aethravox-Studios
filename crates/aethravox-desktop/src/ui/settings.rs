use aethravox_core::{
    ChangePasswordRequest, ColorMode, DeviceProfile, ThemeKind, UpdateProfileRequest,
    UpdateTitlesRequest,
};
use egui::{Color32, Rounding, Stroke};

use crate::api::{
    spawn_change_password, spawn_fetch_profile, spawn_revoke_device, spawn_revoke_session,
    spawn_update_profile, spawn_update_titles,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let mobile = app.is_mobile_layout(ctx);
    let mut refresh_profile = false;
    let mut save_profile = false;
    let mut save_titles = false;
    let mut change_pw = false;
    let mut revoke_session: Option<String> = None;
    let mut revoke_device: Option<String> = None;

    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(!app.settings_tab_profile, "🎨 Apariencia y Pantalla")
            .clicked()
        {
            app.settings_tab_profile = false;
        }
        if ui
            .selectable_label(app.settings_tab_profile, "👤 Perfil, Títulos y Sesiones")
            .clicked()
        {
            app.settings_tab_profile = true;
            if app.profile.is_none() && !app.offline_mode && app.token.is_some() {
                refresh_profile = true;
            }
        }
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("settings_scroll")
        .show(ui, |ui| {
            if !app.settings_tab_profile {
                // Perfil de dispositivo: Desktop, TV, PDI, Móvil
                card_frame(p).show(ui, |ui| {
                    let resolved = app.resolved_device(ctx);
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("📱 Modo de Dispositivo y Pantalla");
                        badge_pill(
                            ui,
                            &format!("Activo: {}", resolved.label()),
                            p.accent_secondary.gamma_multiply(0.18),
                            p.accent_secondary,
                        );
                    });
                    ui.label(
                        egui::RichText::new(
                            "Adapta escala, foco de mando (TV), pizarra táctil (PDI) o diseño vertical (Android/iPhone).",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        for (dp, hint) in [
                            (DeviceProfile::Auto, "🖥 Automático"),
                            (DeviceProfile::Desktop, "💻 Escritorio"),
                            (DeviceProfile::Mobile, "📱 Móvil / iPhone"),
                            (DeviceProfile::Tv, "📺 Smart TV (Mando)"),
                            (DeviceProfile::Pdi, "🖍 PDI (Pizarra Táctil)"),
                        ] {
                            let active = app.device_profile == dp;
                            if ui.selectable_label(active, hint).clicked() {
                                app.set_device_profile(ctx, dp);
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let fs_label = if app.fullscreen {
                            "🗗 Salir de pantalla completa"
                        } else {
                            "⛶ Pantalla completa"
                        };
                        if ui.button(fs_label).clicked() {
                            app.toggle_fullscreen(ctx);
                        }
                    });
                });

                ui.add_space(14.0);

                card_frame(p).show(ui, |ui| {
                    ui.heading("Tema visual y modo de color");
                    ui.label(
                        egui::RichText::new("Elige el estilo y la luminosidad de tu plataforma.")
                            .color(p.text_muted),
                    );
                    ui.add_space(12.0);

                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(app.color_mode == ColorMode::Dark, "🌙 Modo Oscuro")
                            .clicked()
                        {
                            app.set_color_mode(ctx, ColorMode::Dark);
                        }
                        if ui
                            .selectable_label(app.color_mode == ColorMode::Light, "☀ Modo Claro")
                            .clicked()
                        {
                            app.set_color_mode(ctx, ColorMode::Light);
                        }
                    });

                    ui.add_space(14.0);

                    let themes = [
                        (
                            ThemeKind::Aethravox,
                            "Obsidiana + Dorado + Cian",
                            Color32::from_rgb(224, 163, 8),
                            Color32::from_rgb(0, 212, 255),
                        ),
                        (
                            ThemeKind::Shadow,
                            "Obsidiana + Morado Neón + Plata",
                            Color32::from_rgb(147, 51, 234),
                            Color32::from_rgb(216, 180, 254),
                        ),
                        (
                            ThemeKind::CommonCents,
                            "Obsidiana + Verde + Azul",
                            Color32::from_rgb(34, 197, 94),
                            Color32::from_rgb(59, 130, 246),
                        ),
                    ];

                    let render_theme_card =
                        |ui: &mut egui::Ui,
                         kind: ThemeKind,
                         subtitle: &str,
                         c1: Color32,
                         c2: Color32|
                         -> bool {
                            let active = app.theme == kind;
                            let resp = elevated_frame(p)
                                .stroke(Stroke::new(
                                    if active { 2.0_f32 } else { 1.0_f32 },
                                    if active {
                                        p.accent_primary
                                    } else {
                                        p.border_subtle
                                    },
                                ))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.set_min_height(if mobile { 84.0 } else { 110.0 });
                                    ui.horizontal(|ui| {
                                        let (r1, painter) = ui.allocate_painter(
                                            egui::vec2(32.0, 14.0),
                                            egui::Sense::hover(),
                                        );
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                r1.rect.min,
                                                egui::vec2(14.0, 14.0),
                                            ),
                                            Rounding::same(4.0),
                                            c1,
                                        );
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                r1.rect.min + egui::vec2(18.0, 0.0),
                                                egui::vec2(14.0, 14.0),
                                            ),
                                            Rounding::same(4.0),
                                            c2,
                                        );
                                        if active {
                                            ui.label(
                                                egui::RichText::new("✓ Activo")
                                                    .small()
                                                    .strong()
                                                    .color(p.accent_primary),
                                            );
                                        }
                                    });
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(kind.display_name()).strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(subtitle)
                                            .small()
                                            .color(p.text_muted),
                                    );
                                })
                                .response
                                .interact(egui::Sense::click());
                            resp.clicked()
                        };

                    let mut picked_theme: Option<ThemeKind> = None;
                    if mobile {
                        for (kind, subtitle, c1, c2) in themes {
                            if render_theme_card(ui, kind, subtitle, c1, c2) {
                                picked_theme = Some(kind);
                            }
                            ui.add_space(8.0);
                        }
                    } else {
                        ui.columns(3, |cols| {
                            for (idx, (kind, subtitle, c1, c2)) in themes.into_iter().enumerate() {
                                cols[idx].vertical(|ui| {
                                    if render_theme_card(ui, kind, subtitle, c1, c2) {
                                        picked_theme = Some(kind);
                                    }
                                });
                            }
                        });
                    }
                    if let Some(t) = picked_theme {
                        app.set_theme(ctx, t);
                    }
                });

                ui.add_space(14.0);

                if let Some(hub) = &app.hub {
                    card_frame(p).show(ui, |ui| {
                        ui.heading("Resumen de sesión");
                        ui.add_space(6.0);
                        ui.label(format!("Usuario: {}", hub.user.username));
                        ui.label(format!("Rol: {}", hub.user.role_display));
                        ui.label(format!("Servidor: {}", app.server_url));
                    });
                }
                return;
            }

            // Tab Perfil, Títulos y Sesiones
            if let Some(err) = &app.profile_error {
                ui.label(egui::RichText::new(err).small().color(p.danger));
                ui.add_space(6.0);
            }

            if let Some(prof) = app.profile.clone() {
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(format!("👤 @{}", prof.username));
                        badge_pill(
                            ui,
                            &prof.role_display,
                            p.accent_primary.gamma_multiply(0.18),
                            p.accent_primary,
                        );
                        if ui.button("↻ Actualizar").clicked() {
                            refresh_profile = true;
                        }
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "Email: {} · Miembro desde: {} · Último acceso: {} · Uso: {}",
                            prof.email, prof.created_at, prof.last_login, prof.usage
                        ))
                        .small()
                        .color(p.text_muted),
                    );
                    ui.separator();

                    ui.label(egui::RichText::new("Nombre visible").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut app.profile_display_name)
                            .desired_width(ui.available_width().min(340.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Biografía").strong());
                    ui.add(
                        egui::TextEdit::multiline(&mut app.profile_bio)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3),
                    );
                    ui.add_space(8.0);
                    if primary_button(ui, p, "Guardar perfil").clicked() {
                        save_profile = true;
                    }
                });

                ui.add_space(12.0);

                // Títulos
                card_frame(p).show(ui, |ui| {
                    ui.heading("🏅 Títulos (máx. 3 visibles)");
                    ui.separator();
                    if prof.titles.is_empty() {
                        ui.label(
                            egui::RichText::new("No has desbloqueado títulos todavía.")
                                .small()
                                .color(p.text_muted),
                        );
                    } else {
                        for t in &prof.titles {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let mut vis = app.profile_selected_titles.contains(&t.id);
                                    if ui.checkbox(&mut vis, &t.name).changed() {
                                        if vis {
                                            if app.profile_selected_titles.len() < 3
                                                && !app.profile_selected_titles.contains(&t.id)
                                            {
                                                app.profile_selected_titles.push(t.id);
                                            }
                                        } else {
                                            app.profile_selected_titles.retain(|&id| id != t.id);
                                        }
                                    }
                                    badge_pill(
                                        ui,
                                        &t.effect,
                                        p.accent_secondary.gamma_multiply(0.16),
                                        p.accent_secondary,
                                    );
                                    if !t.description.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&t.description)
                                                .small()
                                                .color(p.text_muted),
                                        );
                                    }
                                    let is_prim = app.profile_primary_title == Some(t.id);
                                    if ui.selectable_label(is_prim, "Principal").clicked() {
                                        app.profile_primary_title = if is_prim {
                                            None
                                        } else {
                                            Some(t.id)
                                        };
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                        if primary_button(ui, p, "Guardar títulos").clicked() {
                            save_titles = true;
                        }
                    }
                });

                ui.add_space(12.0);

                // Cambiar contraseña
                card_frame(p).show(ui, |ui| {
                    ui.heading("🔑 Cambiar contraseña");
                    ui.horizontal_wrapped(|ui| {
                        let fw = ui.available_width().min(220.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut app.pw_current)
                                .desired_width(fw)
                                .password(true)
                                .hint_text("Contraseña actual"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut app.pw_new)
                                .desired_width(fw)
                                .password(true)
                                .hint_text("Nueva contraseña"),
                        );
                        if primary_button(ui, p, "Actualizar").clicked()
                            && !app.pw_current.is_empty()
                            && !app.pw_new.is_empty()
                        {
                            change_pw = true;
                        }
                    });
                });

                ui.add_space(12.0);

                // Sesiones activas
                card_frame(p).show(ui, |ui| {
                    ui.heading(format!("🖥 Sesiones activas ({})", prof.sessions.len()));
                    ui.separator();
                    for s in &prof.sessions {
                        elevated_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(format!("{}…", s.token_prefix)).strong());
                                if s.is_current {
                                    badge_pill(
                                        ui,
                                        "Actual",
                                        p.success.gamma_multiply(0.18),
                                        p.success,
                                    );
                                }
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Creada: {} · Última vista: {} · Expira: {}",
                                        s.created_at, s.last_seen, s.expires
                                    ))
                                    .small()
                                    .color(p.text_muted),
                                );
                                if !s.is_current
                                    && ui
                                        .button(egui::RichText::new("Revocar").color(p.danger))
                                        .clicked()
                                {
                                    revoke_session = Some(s.token.clone());
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }
                });

                if !prof.linked_devices.is_empty() {
                    ui.add_space(12.0);
                    card_frame(p).show(ui, |ui| {
                        ui.heading(format!(
                            "📱 Dispositivos vinculados ({})",
                            prof.linked_devices.len()
                        ));
                        ui.separator();
                        for d in &prof.linked_devices {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&d.device_name).strong());
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Última vista: {} · Expira: {}",
                                            d.last_seen, d.expires
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                    if ui
                                        .button(
                                            egui::RichText::new("Desvincular").color(p.danger),
                                        )
                                        .clicked()
                                    {
                                        revoke_device = Some(d.token.clone());
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                }
            } else {
                card_frame(p).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(if app.offline_mode || app.token.is_none() {
                            "Inicia sesión conectado al servidor para editar perfil, títulos y sesiones."
                        } else {
                            "Cargando perfil…"
                        })
                        .color(p.text_muted),
                    );
                });
            }
        });

    if refresh_profile && !app.offline_mode && app.token.is_some() {
        spawn_fetch_profile(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if save_profile && !app.offline_mode && app.token.is_some() {
        spawn_update_profile(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            UpdateProfileRequest {
                display_name: app.profile_display_name.clone(),
                bio: app.profile_bio.clone(),
            },
        );
    }
    if save_titles && !app.offline_mode && app.token.is_some() {
        spawn_update_titles(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            UpdateTitlesRequest {
                primary_id: app.profile_primary_title,
                visible_ids: app.profile_selected_titles.clone(),
            },
        );
    }
    if change_pw && !app.offline_mode && app.token.is_some() {
        let req = ChangePasswordRequest {
            current: app.pw_current.clone(),
            new_password: app.pw_new.clone(),
        };
        app.pw_current.clear();
        app.pw_new.clear();
        spawn_change_password(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            req,
        );
    }
    if let Some(tok) = revoke_session {
        spawn_revoke_session(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            tok,
        );
    }
    if let Some(tok) = revoke_device {
        spawn_revoke_device(
            &app.rt,
            app.tx.clone(),
            ctx,
            app.server_url.clone(),
            app.token.clone(),
            tok,
        );
    }
}
