use aethravox_core::{
    ColorMode, LinkConsumeRequest, LoginRequest, RecoverRequest, RegisterRequest,
    ResetPasswordRequest,
};
use egui::Rounding;

use crate::api::{spawn_link_consume, spawn_login, spawn_recover, spawn_register, spawn_reset};
use crate::app::AethravoxApp;
use crate::theme::{card_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let mobile = app.is_mobile_layout(ctx);
    egui::ScrollArea::vertical().id_salt("login_scroll").show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(if mobile { 16.0 } else { 42.0 });

            ui.label(
                egui::RichText::new("AETHRAVOX STUDIOS")
                    .size(if mobile { 23.0 } else { 28.0 })
                    .strong()
                    .color(p.accent_primary),
            );
            ui.label(
                egui::RichText::new("AI Engineers · App Dev · Games")
                    .small()
                    .color(p.text_muted),
            );
            ui.add_space(if mobile { 12.0 } else { 22.0 });

            let card_w = (ui.available_width() - 24.0).clamp(260.0, 420.0);
            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                .rounding(Rounding::same(14.0))
                .inner_margin(egui::Margin::same(if mobile { 14.0 } else { 22.0 }))
                .show(ui, |ui| {
                    ui.set_width(card_w);

                    ui.horizontal_wrapped(|ui| {
                        for (mode, lbl) in [
                            (0u8, "Entrar"),
                            (1u8, "Registro"),
                            (2u8, "Recuperar"),
                            (3u8, "Vincular"),
                        ] {
                            if ui.selectable_label(app.auth_mode == mode, lbl).clicked() {
                                app.auth_mode = mode;
                                app.auth_tab_register = mode == 1;
                                app.auth_error = None;
                                app.auth_info = None;
                            }
                        }
                    });

                    ui.separator();
                    ui.add_space(6.0);

                    if let Some(err) = &app.auth_error {
                        egui::Frame::none()
                            .fill(p.danger.gamma_multiply(0.16))
                            .stroke(egui::Stroke::new(1.0_f32, p.danger))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(err).color(p.danger).small());
                            });
                        ui.add_space(8.0);
                    }

                    if let Some(info) = &app.auth_info {
                        egui::Frame::none()
                            .fill(p.success.gamma_multiply(0.14))
                            .stroke(egui::Stroke::new(1.0_f32, p.success))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(info).color(p.success).small());
                            });
                        ui.add_space(8.0);
                    }

                    ui.label(egui::RichText::new("Servidor API").small().color(p.text_muted));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.server_url)
                            .desired_width(f32::INFINITY)
                            .hint_text("https://aethravox-studios.duckdns.org"),
                    );
                    ui.add_space(6.0);

                    match app.auth_mode {
                        0 | 1 => {
                            let is_reg = app.auth_mode == 1;
                            ui.label(egui::RichText::new("Usuario").small().color(p.text_muted));
                            ui.add(
                                egui::TextEdit::singleline(&mut app.login_username)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Tu usuario"),
                            );

                            if is_reg {
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new("Correo AethraMail")
                                        .small()
                                        .color(p.text_muted),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.register_email)
                                        .desired_width(f32::INFINITY)
                                        .hint_text("usuario@aethra.sac"),
                                );
                            }

                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Contraseña").small().color(p.text_muted),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.login_password)
                                    .password(true)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("••••••••"),
                            );

                            ui.add_space(12.0);
                            ui.add_enabled_ui(!app.auth_busy, |ui| {
                                let btn_label = if app.auth_busy {
                                    "Conectando…"
                                } else if is_reg {
                                    "Crear cuenta"
                                } else {
                                    "Entrar al Estudio"
                                };

                                if primary_button(ui, p, btn_label).clicked() {
                                    app.auth_busy = true;
                                    app.auth_error = None;
                                    app.auth_info = None;
                                    if is_reg {
                                        spawn_register(
                                            &app.rt,
                                            app.tx.clone(),
                                            ctx.clone(),
                                            app.server_url.clone(),
                                            RegisterRequest {
                                                username: app.login_username.trim().to_string(),
                                                email: app.register_email.trim().to_string(),
                                                password: app.login_password.clone(),
                                            },
                                        );
                                    } else {
                                        spawn_login(
                                            &app.rt,
                                            app.tx.clone(),
                                            ctx.clone(),
                                            app.server_url.clone(),
                                            LoginRequest {
                                                username: app.login_username.trim().to_string(),
                                                password: app.login_password.clone(),
                                                remember: true,
                                            },
                                        );
                                    }
                                }
                            });
                        }
                        2 => {
                            // Recuperación y restablecimiento de contraseña
                            ui.label(
                                egui::RichText::new("1. Solicitar token de recuperación")
                                    .strong()
                                    .small(),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.recover_email)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Tu email registrado"),
                            );
                            ui.add_space(6.0);
                            ui.add_enabled_ui(!app.auth_busy, |ui| {
                                if ui.button("Enviar instrucciones").clicked()
                                    && !app.recover_email.trim().is_empty()
                                {
                                    app.auth_busy = true;
                                    app.auth_error = None;
                                    spawn_recover(
                                        &app.rt,
                                        app.tx.clone(),
                                        ctx.clone(),
                                        app.server_url.clone(),
                                        RecoverRequest {
                                            email: app.recover_email.trim().to_string(),
                                        },
                                    );
                                }
                            });

                            ui.separator();
                            ui.label(
                                egui::RichText::new("2. Restablecer con token")
                                    .strong()
                                    .small(),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.reset_token)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Token de recuperación"),
                            );
                            ui.add_space(4.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut app.reset_new_password)
                                    .password(true)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Nueva contraseña"),
                            );
                            ui.add_space(8.0);
                            ui.add_enabled_ui(!app.auth_busy, |ui| {
                                if primary_button(ui, p, "Cambiar contraseña").clicked()
                                    && !app.reset_token.trim().is_empty()
                                    && !app.reset_new_password.is_empty()
                                {
                                    app.auth_busy = true;
                                    app.auth_error = None;
                                    spawn_reset(
                                        &app.rt,
                                        app.tx.clone(),
                                        ctx.clone(),
                                        app.server_url.clone(),
                                        ResetPasswordRequest {
                                            token: app.reset_token.trim().to_string(),
                                            new_password: app.reset_new_password.clone(),
                                        },
                                    );
                                }
                            });
                        }
                        _ => {
                            // Vincular dispositivo con código de 6 dígitos
                            ui.label(
                                egui::RichText::new(
                                    "Introduce el código de 6 dígitos generado desde otro dispositivo ya conectado:",
                                )
                                .small()
                                .color(p.text_muted),
                            );
                            ui.add_space(6.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut app.link_code_input)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Código de 6 dígitos o token QR"),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Nombre de este dispositivo")
                                    .small()
                                    .color(p.text_muted),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut app.link_device_name)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Ej. Mi Android / Tablet"),
                            );
                            ui.add_space(10.0);
                            ui.add_enabled_ui(!app.auth_busy, |ui| {
                                if primary_button(ui, p, "🔗 Vincular ahora").clicked()
                                    && !app.link_code_input.trim().is_empty()
                                {
                                    app.auth_busy = true;
                                    app.auth_error = None;
                                    spawn_link_consume(
                                        &app.rt,
                                        app.tx.clone(),
                                        ctx.clone(),
                                        app.server_url.clone(),
                                        LinkConsumeRequest {
                                            code: app.link_code_input.trim().to_string(),
                                            device_name: Some(app.link_device_name.trim().to_string()),
                                        },
                                    );
                                }
                            });
                        }
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Modo local").clicked() {
                            let name: String = if app.login_username.trim().is_empty() {
                                "Aethravox".to_string()
                            } else {
                                app.login_username.trim().to_string()
                            };
                            app.enter_offline_demo(&name, false);
                        }
                        if ui.button("Invitado").clicked() {
                            app.enter_offline_demo("Invitado", true);
                        }
                        let icon = match app.color_mode {
                            ColorMode::Dark => "☀ Claro",
                            ColorMode::Light => "🌙 Oscuro",
                        };
                        if ui.button(icon).clicked() {
                            let next = match app.color_mode {
                                ColorMode::Dark => ColorMode::Light,
                                ColorMode::Light => ColorMode::Dark,
                            };
                            app.set_color_mode(ctx, next);
                        }
                        if ui.selectable_label(app.vk_manual_open, "⌨ Teclado").clicked() {
                            app.vk_manual_open = !app.vk_manual_open;
                            if app.vk_manual_open {
                                app.vk_dismissed = false;
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Privacidad").clicked() {
                            app.legal_modal = Some("privacidad");
                        }
                        if ui.small_button("Términos de uso").clicked() {
                            app.legal_modal = Some("terminos");
                        }
                    });
                });

            let _ = card_frame;
        });
    });

    if let Some(kind) = app.legal_modal {
        let mut open = true;
        let title = if kind == "privacidad" {
            "Política de Privacidad — Aethravox Studios"
        } else {
            "Términos de Uso — Aethravox Studios"
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(340.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    if kind == "privacidad" {
                        ui.label("Tus datos de sesión, mensajes y perfil se almacenan exclusivamente en el servidor de Aethravox Studios configurado arriba. No compartimos información con terceros ni rastreamos actividad fuera de la plataforma.");
                    } else {
                        ui.label("El uso de Aethravox Studios, Aris IA, Comunidades, aexTube y AethraMail implica respetar las normas de convivencia de la plataforma y no abusar de los recursos ni intentar vulnerar cuentas ajenas.");
                    }
                });
                ui.add_space(10.0);
                if primary_button(ui, p, "Cerrar").clicked() {
                    app.legal_modal = None;
                }
            });
        if !open {
            app.legal_modal = None;
        }
    }
}
