use aethravox_core::{ColorMode, LoginRequest, RegisterRequest};
use egui::Rounding;

use crate::api::{spawn_login, spawn_register};
use crate::app::AethravoxApp;
use crate::theme::{card_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);

        ui.label(
            egui::RichText::new("AETHRAVOX STUDIOS")
                .size(28.0)
                .strong()
                .color(p.accent_primary),
        );
        ui.label(
            egui::RichText::new("AI Engineers · App Dev · Games")
                .small()
                .color(p.text_muted),
        );
        ui.add_space(24.0);

        egui::Frame::none()
            .fill(p.bg_surface)
            .stroke(egui::Stroke::new(1.0, p.border_subtle))
            .rounding(Rounding::same(14.0))
            .inner_margin(egui::Margin::same(24.0))
            .show(ui, |ui| {
                ui.set_width(380.0);

                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(!app.auth_tab_register, "Iniciar sesión")
                        .clicked()
                    {
                        app.auth_tab_register = false;
                        app.auth_error = None;
                    }
                    if ui
                        .selectable_label(app.auth_tab_register, "Crear cuenta")
                        .clicked()
                    {
                        app.auth_tab_register = true;
                        app.auth_error = None;
                    }
                });

                ui.separator();
                ui.add_space(6.0);

                if let Some(err) = &app.auth_error {
                    egui::Frame::none()
                        .fill(p.danger.gamma_multiply(0.16))
                        .stroke(egui::Stroke::new(1.0, p.danger))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(err).color(p.danger).small());
                        });
                    ui.add_space(8.0);
                }

                ui.label(egui::RichText::new("Servidor API").small().color(p.text_muted));
                ui.add(
                    egui::TextEdit::singleline(&mut app.server_url)
                        .desired_width(f32::INFINITY)
                        .hint_text("http://127.0.0.1:8080"),
                );
                ui.add_space(6.0);

                ui.label(egui::RichText::new("Usuario").small().color(p.text_muted));
                ui.add(
                    egui::TextEdit::singleline(&mut app.login_username)
                        .desired_width(f32::INFINITY)
                        .hint_text("Tu usuario"),
                );

                if app.auth_tab_register {
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
                ui.label(egui::RichText::new("Contraseña").small().color(p.text_muted));
                ui.add(
                    egui::TextEdit::singleline(&mut app.login_password)
                        .password(true)
                        .desired_width(f32::INFINITY)
                        .hint_text("••••••••"),
                );

                ui.add_space(14.0);

                ui.add_enabled_ui(!app.auth_busy, |ui| {
                    let btn_label = if app.auth_busy {
                        "Conectando…"
                    } else if app.auth_tab_register {
                        "Crear cuenta"
                    } else {
                        "Entrar al Estudio"
                    };

                    if primary_button(ui, p, btn_label).clicked() {
                        app.auth_busy = true;
                        app.auth_error = None;
                        if app.auth_tab_register {
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

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Modo local (Preview)").clicked() {
                        let name = if app.login_username.trim().is_empty() {
                            "Aethravox"
                        } else {
                            app.login_username.trim()
                        };
                        app.enter_offline_demo(name, false);
                    }
                    if ui.button("Entrar como Invitado").clicked() {
                        app.enter_offline_demo("Invitado", true);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                    });
                });
            });

        let _ = card_frame;
    });
}
