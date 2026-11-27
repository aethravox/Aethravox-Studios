use aethravox_core::{
    ChangePasswordRequest, ColorMode, DeviceProfile, ThemeKind, UpdateProfileRequest,
    UpdateTitlesRequest,
};
use egui::{Color32, Rounding, Stroke};

use crate::api::{
    spawn_change_password, spawn_fetch_emails, spawn_fetch_profile, spawn_link_code,
    spawn_logout_all, spawn_remove_avatar, spawn_remove_email, spawn_request_email,
    spawn_create_zlib_token, spawn_fetch_zlibs_mine, spawn_revoke_device, spawn_revoke_session,
    spawn_revoke_zlib_token, spawn_set_primary_email, spawn_update_profile,
    spawn_update_titles, spawn_upload_avatar_bytes, spawn_upload_avatar_path, spawn_verify_email,
};
use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

#[allow(dead_code)]
enum AvatarAction {
    Bytes { filename: String, bytes: Vec<u8> },
    Path(String),
    Remove,
}

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let mobile = app.is_mobile_layout(ctx);
    let mut refresh_profile = false;
    let mut refresh_emails = false;
    let mut refresh_zlibs = false;
    let mut create_zlib_tok = false;
    let mut revoke_zlib_tok: Option<i64> = None;
    let mut save_profile = false;
    let mut save_titles = false;
    let mut change_pw = false;
    let mut generate_link = false;
    let mut logout_all = false;
    let mut req_email: Option<String> = None;
    let mut verify_email: Option<(String, String)> = None;
    let mut primary_email: Option<i64> = None;
    let mut remove_email: Option<i64> = None;
    let mut revoke_session: Option<String> = None;
    let mut revoke_device: Option<String> = None;
    let mut avatar_action: Option<AvatarAction> = None;

    if app.settings_tab_profile {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(f) = dropped.into_iter().next() {
            if let Some(bytes) = f.bytes {
                let name = if f.name.is_empty() {
                    "avatar.png".to_string()
                } else {
                    f.name.clone()
                };
                app.avatar_dropped_name = Some(name);
                app.avatar_dropped_bytes = Some(bytes.to_vec());
            } else if let Some(path) = f.path {
                app.avatar_path = path.display().to_string();
                app.avatar_dropped_name = None;
                app.avatar_dropped_bytes = None;
            }
        }
    }

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
            if !app.offline_mode && app.token.is_some() {
                if app.profile.is_none() {
                    refresh_profile = true;
                }
                if app.emails_payload.is_none() {
                    refresh_emails = true;
                }
                if app.zlibs_account.is_none() {
                    refresh_zlibs = true;
                }
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

                    ui.separator();
                    ui.label(egui::RichText::new("🖼 Avatar de perfil (máx. 5 MB · PNG/JPG/GIF/WEBP)").strong());
                    ui.horizontal_wrapped(|ui| {
                        if prof.avatar_url.is_empty() {
                            ui.label(
                                egui::RichText::new("Sin avatar personalizado")
                                    .small()
                                    .color(p.text_muted),
                            );
                        } else {
                            if ui.button("👁 Abrir avatar actual ↗").clicked() {
                                let url = format!(
                                    "{}{}",
                                    app.server_url.trim_end_matches('/'),
                                    prof.avatar_url
                                );
                                ctx.open_url(egui::OpenUrl::new_tab(url));
                            }
                            if !app.avatar_busy
                                && ui
                                    .button(egui::RichText::new("🗑 Quitar avatar").color(p.danger))
                                    .clicked()
                            {
                                avatar_action = Some(AvatarAction::Remove);
                            }
                        }
                    });
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut app.avatar_path)
                                .desired_width(ui.available_width().min(360.0))
                                .hint_text("Arrastra una imagen aquí o pega la ruta local…"),
                        );
                    }
                    #[cfg(target_arch = "wasm32")]
                    ui.label(
                        egui::RichText::new("Arrastra una imagen directamente sobre esta ventana.")
                            .small()
                            .color(p.text_muted),
                    );
                    if let Some(name) = &app.avatar_dropped_name {
                        ui.label(
                            egui::RichText::new(format!("Imagen lista: {name}"))
                                .small()
                                .color(p.accent_secondary),
                        );
                    }
                    ui.add_space(4.0);
                    if app.avatar_busy {
                        ui.label(
                            egui::RichText::new("⏳ Subiendo avatar…")
                                .small()
                                .color(p.accent_secondary),
                        );
                    } else {
                        let can_upload = app.avatar_dropped_bytes.is_some()
                            || !app.avatar_path.trim().is_empty();
                        if can_upload && primary_button(ui, p, "📷 Subir avatar").clicked() {
                            if let (Some(fname), Some(bytes)) = (
                                app.avatar_dropped_name.clone(),
                                app.avatar_dropped_bytes.clone(),
                            ) {
                                avatar_action = Some(AvatarAction::Bytes {
                                    filename: fname,
                                    bytes,
                                });
                            } else {
                                avatar_action = Some(AvatarAction::Path(app.avatar_path.clone()));
                            }
                        }
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

                // Vincular otro dispositivo
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("🔗 Vincular otro dispositivo");
                        if primary_button(ui, p, "Generar código (60s)").clicked() {
                            generate_link = true;
                        }
                    });
                    ui.label(
                        egui::RichText::new(
                            "Úsalo desde la pestaña «Vincular» en otro dispositivo (móvil, TV o PC) sin escribir contraseña.",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    if let Some(lc) = &app.link_code_generated {
                        ui.add_space(8.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("Código: {}", lc.numeric_code))
                                    .size(22.0)
                                    .strong()
                                    .color(p.accent_primary),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Token completo: {} · Cuenta: @{} · Expira: {}",
                                    lc.code, lc.target_username, lc.expires_at
                                ))
                                .small()
                                .color(p.text_muted),
                            );
                        });
                    }
                });

                ui.add_space(12.0);

                // Correo interno (AethraMail)
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("📧 Correo interno (AethraMail)");
                        if ui.button("↻ Actualizar").clicked() {
                            refresh_emails = true;
                        }
                        if let Some(ep) = &app.emails_payload {
                            if let Some(url) = &ep.aethramail_url {
                                ui.hyperlink_to("Abrir AethraMail ↗", url);
                            }
                        }
                    });
                    if let Some(err) = &app.email_error {
                        ui.label(egui::RichText::new(err).small().color(p.danger));
                    }
                    if let Some(info) = &app.email_info {
                        ui.label(egui::RichText::new(info).small().color(p.success));
                    }
                    ui.separator();

                    if let Some(ep) = app.emails_payload.clone() {
                        if ep.emails.is_empty() {
                            ui.label(
                                egui::RichText::new("No tienes correos internos vinculados todavía.")
                                    .small()
                                    .color(p.text_muted),
                            );
                        } else {
                            for em in ep.emails {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(&em.email).strong());
                                        if em.verified {
                                            badge_pill(
                                                ui,
                                                "✓ Verificado",
                                                p.success.gamma_multiply(0.18),
                                                p.success,
                                            );
                                        } else {
                                            badge_pill(
                                                ui,
                                                "Pendiente",
                                                p.accent_secondary.gamma_multiply(0.18),
                                                p.accent_secondary,
                                            );
                                        }
                                        if em.is_primary {
                                            badge_pill(
                                                ui,
                                                "Primario",
                                                p.accent_primary.gamma_multiply(0.18),
                                                p.accent_primary,
                                            );
                                        } else {
                                            if em.verified && ui.button("Hacer primario").clicked() {
                                                primary_email = Some(em.id);
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new("Eliminar").color(p.danger),
                                                )
                                                .clicked()
                                            {
                                                remove_email = Some(em.id);
                                            }
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Vincular correo AethraMail").strong().small());
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.email_input)
                                .desired_width(ui.available_width().min(250.0))
                                .hint_text("usuario@aethra.sac"),
                        );
                        if primary_button(ui, p, "Enviar código (8 dígitos)").clicked()
                            && !app.email_input.trim().is_empty()
                        {
                            req_email = Some(app.email_input.trim().to_string());
                        }
                    });

                    if let Some(pending) = app.email_pending_verify.clone() {
                        ui.add_space(6.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Introduce el código de 8 dígitos enviado a {pending}:"
                                ))
                                .small(),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.email_verify_code)
                                        .desired_width(160.0)
                                        .hint_text("12345678"),
                                );
                                if primary_button(ui, p, "Verificar").clicked()
                                    && !app.email_verify_code.trim().is_empty()
                                {
                                    verify_email = Some((
                                        pending.clone(),
                                        app.email_verify_code.trim().to_string(),
                                    ));
                                }
                                if ui.button("Cancelar").clicked() {
                                    app.email_pending_verify = None;
                                    app.email_verify_code.clear();
                                }
                            });
                        });
                    }
                });

                ui.add_space(12.0);

                // Sesiones activas
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(format!("🖥 Sesiones activas ({})", prof.sessions.len()));
                        if ui
                            .button(
                                egui::RichText::new("Cerrar todas las sesiones").color(p.danger),
                            )
                            .clicked()
                        {
                            logout_all = true;
                        }
                    });
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

                ui.add_space(12.0);
                // Zen CLI & ZLibs
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.heading("📦 Zen CLI y ZLibs");
                        if ui.button("↻ Actualizar").clicked() {
                            refresh_zlibs = true;
                        }
                        if ui.button("Abrir ZLibs ↗").clicked() {
                            app.navigate_to(ctx, Route::ZLibs);
                        }
                    });
                    ui.label(
                        egui::RichText::new(
                            "Genera tu token CLI (almacenado con SHA-256) y consulta tus librerías Zen publicadas en GitHub.",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    if let Some(err) = &app.zlibs_error {
                        ui.label(egui::RichText::new(format!("⚠️ {err}")).small().color(p.danger));
                    }
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.zlibs_token_label)
                                .desired_width(200.0)
                                .hint_text("Etiqueta (ej: Mi PC)"),
                        );
                        if primary_button(ui, p, "🔑 Generar Token CLI").clicked() {
                            create_zlib_tok = true;
                        }
                    });
                    if let Some(created) = &app.zlibs_created_token {
                        ui.add_space(8.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Token CLI creado (cópialo ahora):")
                                    .strong()
                                    .color(p.accent_primary),
                            );
                            ui.monospace(&created.cli_command);
                            if primary_button(ui, p, "📋 Copiar zen login").clicked() {
                                ctx.copy_text(created.cli_command.clone());
                                app.status_banner = Some("Comando zen login copiado.".into());
                            }
                        });
                    }
                    if let Some(acc) = &app.zlibs_account {
                        if !acc.tokens.is_empty() {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Tokens CLI activos:").strong().small());
                            for t in &acc.tokens {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(&t.label).strong());
                                        ui.monospace(&t.token_prefix);
                                        if ui
                                            .button(egui::RichText::new("Revocar").color(p.danger))
                                            .clicked()
                                        {
                                            revoke_zlib_tok = Some(t.id);
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        }
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Paquetes publicados ({}):",
                                acc.my_packages.len()
                            ))
                            .strong()
                            .small(),
                        );
                        if acc.my_packages.is_empty() {
                            ui.label(
                                egui::RichText::new("Todavía no has publicado librerías Zen.")
                                    .small()
                                    .color(p.text_muted),
                            );
                        } else {
                            for pkg in &acc.my_packages {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(&pkg.name)
                                                .strong()
                                                .color(p.accent_primary),
                                        );
                                        if ui.button("GitHub ↗").clicked() {
                                            ctx.open_url(egui::OpenUrl::new_tab(pkg.git_url.clone()));
                                        }
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        for v in &pkg.versions {
                                            if ui.button(format!("v{}", v.version)).clicked() {
                                                ctx.copy_text(v.dep_line.clone());
                                                app.status_banner =
                                                    Some(format!("Copiado: {}", v.dep_line));
                                            }
                                        }
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        ui.monospace(&pkg.dep_line);
                                        if ui.button("📋 Copiar Make.mz").clicked() {
                                            ctx.copy_text(pkg.dep_line.clone());
                                            app.status_banner =
                                                Some(format!("Copiado: {}", pkg.dep_line));
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        }
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
    if refresh_zlibs && !app.offline_mode && app.token.is_some() {
        spawn_fetch_zlibs_mine(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if create_zlib_tok && !app.offline_mode && app.token.is_some() {
        spawn_create_zlib_token(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            app.zlibs_token_label.clone(),
        );
    }
    if let Some(tid) = revoke_zlib_tok {
        if !app.offline_mode && app.token.is_some() {
            spawn_revoke_zlib_token(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                tid,
            );
        }
    }
    if refresh_emails && !app.offline_mode && app.token.is_some() {
        spawn_fetch_emails(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if generate_link && !app.offline_mode && app.token.is_some() {
        spawn_link_code(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if logout_all && !app.offline_mode && app.token.is_some() {
        spawn_logout_all(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if let Some(email) = req_email {
        if !app.offline_mode && app.token.is_some() {
            spawn_request_email(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                email,
            );
        }
    }
    if let Some((email, code)) = verify_email {
        if !app.offline_mode && app.token.is_some() {
            spawn_verify_email(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                email,
                code,
            );
        }
    }
    if let Some(eid) = primary_email {
        if !app.offline_mode && app.token.is_some() {
            spawn_set_primary_email(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                eid,
            );
        }
    }
    if let Some(eid) = remove_email {
        if !app.offline_mode && app.token.is_some() {
            spawn_remove_email(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                eid,
            );
        }
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
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            tok,
        );
    }
    if let Some(act) = avatar_action {
        if !app.offline_mode && app.token.is_some() {
            app.avatar_busy = true;
            match act {
                AvatarAction::Bytes { filename, bytes } => {
                    spawn_upload_avatar_bytes(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        filename,
                        String::new(),
                        bytes,
                    );
                }
                AvatarAction::Path(path) => {
                    spawn_upload_avatar_path(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                        path,
                    );
                }
                AvatarAction::Remove => {
                    spawn_remove_avatar(
                        &app.rt,
                        app.tx.clone(),
                        ctx.clone(),
                        app.server_url.clone(),
                        app.token.clone(),
                    );
                }
            }
        }
    }
}
