use aethravox_core::{ColorMode, DeviceProfile, ThemeKind};
use egui::{Color32, Rounding};

use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, card_frame, parse_hex_color, Palette};

fn all_nav_items(can_admin: bool) -> Vec<(&'static str, &'static str, Route)> {
    let mut items = vec![
        ("🏛", "Inicio (Hub)", Route::Hub),
        ("✨", "Aris IA", Route::Aris),
        ("💬", "Mensajería", Route::Chat),
        ("👥", "Amigos", Route::Friends),
        ("🌐", "Comunidades", Route::Communities),
        ("📄", "Documentos", Route::Docs),
        ("🎨", "Neon / Pizarra PDI", Route::NeonCanvas),
        ("📺", "aexTube", Route::AexTube),
        ("🧪", "Playground", Route::Playground),
        ("🖥", "aexOS", Route::Aexos),
        ("📡", "Transmitir", Route::Transmit),
        ("⬇", "Descargas", Route::Downloads),
        ("🛟", "Asistencia", Route::Support),
        ("⚙", "Perfil y Tema", Route::Settings),
    ];
    if can_admin {
        items.push(("🛡", "Admin", Route::Admin));
    }
    items
}

pub fn show_topbar(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let mobile = app.is_mobile_layout(ctx);
    let resolved = app.resolved_device(ctx);

    ui.horizontal_centered(|ui| {
        if !mobile {
            let toggle_ico = if app.sidebar_collapsed { "▶" } else { "◀" };
            if ui.button(toggle_ico).clicked() {
                app.sidebar_collapsed = !app.sidebar_collapsed;
            }
        }

        ui.label(
            egui::RichText::new(if mobile { "Aethravox" } else { "Aethravox Studios" })
                .strong()
                .size(if mobile { 16.0 } else { 17.0 })
                .color(p.accent_primary),
        );

        if !mobile {
            ui.label(egui::RichText::new("/").color(p.text_muted));
            ui.label(
                egui::RichText::new("AI Engineers · App Dev · Games")
                    .small()
                    .color(p.text_muted),
            );
        }

        if app.offline_mode && !mobile {
            badge_pill(ui, "Preview local", p.bg_elevated, p.accent_secondary);
        }

        if !mobile {
            match resolved {
                DeviceProfile::Tv => badge_pill(ui, "📺 TV", p.bg_elevated, p.accent_primary),
                DeviceProfile::Pdi => badge_pill(ui, "🧑‍🏫 PDI", p.bg_elevated, p.accent_secondary),
                _ => {}
            }
        }

        if let Some(banner) = app.status_banner.clone() {
            if !mobile {
                badge_pill(
                    ui,
                    &banner,
                    p.accent_primary.gamma_multiply(0.16),
                    p.accent_primary,
                );
                if ui.small_button("✕").clicked() {
                    app.status_banner = None;
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if mobile { "Salir" } else { "Cerrar sesión" }).clicked() {
                app.logout();
            }

            if !mobile {
                if let Some(hub) = &app.hub {
                    let role_bg = parse_hex_color(&hub.user.role_color, p.accent_primary);
                    let role_fg = parse_hex_color(&hub.user.role_text_color, Color32::WHITE);
                    badge_pill(ui, &hub.user.role_display, role_bg, role_fg);
                    ui.label(egui::RichText::new(&hub.user.display).strong());
                }

                let fs_ico = if app.fullscreen { "🗗" } else { "⛶" };
                if ui
                    .button(fs_ico)
                    .on_hover_text("Pantalla completa (ideal TV / PDI)")
                    .clicked()
                {
                    app.toggle_fullscreen(ctx);
                }
            }

            let vk_active = app.vk_manual_open;
            if ui
                .selectable_label(vk_active, "⌨")
                .on_hover_text("Mostrar / ocultar teclado en pantalla")
                .clicked()
            {
                app.vk_manual_open = !app.vk_manual_open;
                if app.vk_manual_open {
                    app.vk_dismissed = false;
                }
            }

            let mode_label = match app.color_mode {
                ColorMode::Dark => "☀",
                ColorMode::Light => "🌙",
            };
            if ui
                .button(mode_label)
                .on_hover_text("Cambiar modo claro / oscuro")
                .clicked()
            {
                let next = match app.color_mode {
                    ColorMode::Dark => ColorMode::Light,
                    ColorMode::Light => ColorMode::Dark,
                };
                app.set_color_mode(ctx, next);
            }

            if !mobile && ui.button("✨ Aris").clicked() {
                app.navigate_to(ctx, Route::Aris);
            }
        });
    });
}

pub fn show_sidebar(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let compact = app.sidebar_collapsed;
    let can_admin = app
        .hub
        .as_ref()
        .map(|h| h.user.can_access_admin)
        .unwrap_or(false);

    let nav_items = all_nav_items(can_admin);
    let mut clicked_route: Option<Route> = None;

    ui.vertical(|ui| {
        let bottom_reserve = if compact { 12.0 } else { 112.0 };
        let scroll_h = (ui.available_height() - bottom_reserve).max(160.0);
        egui::ScrollArea::vertical()
            .id_salt("sidebar_nav_scroll")
            .max_height(scroll_h)
            .show(ui, |ui| {
                ui.add_space(4.0);
                for (ico, label, target) in nav_items {
                    let active = app.route == target;
                    let text = if compact {
                        ico.to_string()
                    } else {
                        format!("{ico}  {label}")
                    };
                    let btn = egui::Button::new(
                        egui::RichText::new(text).color(if active {
                            p.accent_primary
                        } else {
                            p.text_primary
                        }),
                    )
                    .fill(if active {
                        p.accent_primary.gamma_multiply(0.14)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(if active {
                        egui::Stroke::new(1.0_f32, p.accent_primary.gamma_multiply(0.5))
                    } else {
                        egui::Stroke::NONE
                    })
                    .rounding(Rounding::same(8.0))
                    .min_size(egui::vec2(ui.available_width(), 34.0));

                    if ui.add(btn).clicked() {
                        clicked_route = Some(target);
                    }
                }
            });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            if !compact {
                ui.horizontal(|ui| {
                    let modes = [
                        (DeviceProfile::Auto, "Auto"),
                        (DeviceProfile::Tv, "TV"),
                        (DeviceProfile::Pdi, "PDI"),
                        (DeviceProfile::Mobile, "Móvil"),
                    ];
                    for (dp, name) in modes {
                        let sel = app.device_profile == dp;
                        if ui.selectable_label(sel, egui::RichText::new(name).small()).clicked() {
                            app.set_device_profile(ctx, dp);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    let themes = [
                        (ThemeKind::Aethravox, "Aethra", Color32::from_rgb(224, 163, 8)),
                        (ThemeKind::Shadow, "Shadow", Color32::from_rgb(147, 51, 234)),
                        (ThemeKind::CommonCents, "Cents", Color32::from_rgb(34, 197, 94)),
                    ];
                    for (t, name, swatch) in themes {
                        let selected = app.theme == t;
                        let btn = egui::Button::new(
                            egui::RichText::new(name).small().color(if selected {
                                swatch
                            } else {
                                p.text_muted
                            }),
                        )
                        .rounding(Rounding::same(6.0));
                        if ui.add(btn).clicked() {
                            app.set_theme(ctx, t);
                        }
                    }
                });
                ui.label(egui::RichText::new("Tema y modo pantalla").small().color(p.text_muted));
                ui.separator();
            }
        });
    });

    if let Some(target) = clicked_route {
        app.navigate_to(ctx, target);
    }
}

pub fn show_mobile_bottom_bar(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &Palette,
) {
    let quick: &[(&str, &str, Route)] = &[
        ("🏛", "Hub", Route::Hub),
        ("✨", "Aris", Route::Aris),
        ("💬", "Chat", Route::Chat),
        ("🎨", "Pizarra", Route::NeonCanvas),
    ];

    let mut nav_to: Option<Route> = None;

    ui.columns(5, |cols| {
        for (i, (ico, lbl, target)) in quick.iter().enumerate() {
            let active = !app.mobile_menu_open && app.route == *target;
            cols[i].vertical_centered(|ui| {
                let btn = egui::Button::new(
                    egui::RichText::new(format!("{ico}\n{lbl}"))
                        .small()
                        .color(if active {
                            p.accent_primary
                        } else {
                            p.text_primary
                        }),
                )
                .fill(if active {
                    p.accent_primary.gamma_multiply(0.14)
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(ui.available_width(), 44.0));
                if ui.add(btn).clicked() {
                    nav_to = Some(*target);
                }
            });
        }
        cols[4].vertical_centered(|ui| {
            let active = app.mobile_menu_open;
            let btn = egui::Button::new(
                egui::RichText::new("☰\nMenú")
                    .small()
                    .color(if active {
                        p.accent_primary
                    } else {
                        p.text_primary
                    }),
            )
            .fill(if active {
                p.accent_primary.gamma_multiply(0.14)
            } else {
                Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(ui.available_width(), 44.0));
            if ui.add(btn).clicked() {
                app.mobile_menu_open = !app.mobile_menu_open;
            }
        });
    });

    if let Some(target) = nav_to {
        app.navigate_to(ctx, target);
    }
}

pub fn show_mobile_drawer(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &Palette,
) {
    let can_admin = app
        .hub
        .as_ref()
        .map(|h| h.user.can_access_admin)
        .unwrap_or(false);
    let nav_items = all_nav_items(can_admin);
    let mut clicked: Option<Route> = None;

    egui::ScrollArea::vertical()
        .id_salt("mobile_drawer_scroll")
        .show(ui, |ui| {
            card_frame(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Menú completo");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕ Cerrar").clicked() {
                            app.mobile_menu_open = false;
                        }
                    });
                });
                ui.separator();

                let cols = 2;
                egui::Grid::new("mobile_drawer_grid")
                    .num_columns(cols)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        let cell_w = ((ui.available_width() - 8.0) / 2.0).max(120.0);
                        for (idx, (ico, label, target)) in nav_items.into_iter().enumerate() {
                            let active = app.route == target;
                            let btn = egui::Button::new(
                                egui::RichText::new(format!("{ico}  {label}")).color(if active {
                                    p.accent_primary
                                } else {
                                    p.text_primary
                                }),
                            )
                            .fill(if active {
                                p.accent_primary.gamma_multiply(0.16)
                            } else {
                                p.bg_elevated
                            })
                            .rounding(Rounding::same(10.0))
                            .min_size(egui::vec2(cell_w, 44.0));
                            if ui.add(btn).clicked() {
                                clicked = Some(target);
                            }
                            if idx % cols == 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });

    if let Some(target) = clicked {
        app.navigate_to(ctx, target);
    }
}
