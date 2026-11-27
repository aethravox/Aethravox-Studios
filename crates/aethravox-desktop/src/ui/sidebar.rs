use aethravox_core::{ColorMode, ThemeKind};
use egui::{Color32, Rounding};

use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, parse_hex_color, Palette};

pub fn show_topbar(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    ui.horizontal_centered(|ui| {
        let toggle_ico = if app.sidebar_collapsed { "▶" } else { "◀" };
        if ui.button(toggle_ico).clicked() {
            app.sidebar_collapsed = !app.sidebar_collapsed;
        }

        ui.label(
            egui::RichText::new("Aethravox Studios")
                .strong()
                .size(17.0)
                .color(p.accent_primary),
        );
        ui.label(egui::RichText::new("/").color(p.text_muted));
        ui.label(
            egui::RichText::new("AI Engineers · App Dev · Games")
                .small()
                .color(p.text_muted),
        );

        if app.offline_mode {
            badge_pill(ui, "Preview local", p.bg_elevated, p.accent_secondary);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Cerrar sesión").clicked() {
                app.logout();
            }

            if let Some(hub) = &app.hub {
                let role_bg = parse_hex_color(&hub.user.role_color, p.accent_primary);
                let role_fg = parse_hex_color(&hub.user.role_text_color, Color32::WHITE);
                badge_pill(ui, &hub.user.role_display, role_bg, role_fg);
                ui.label(egui::RichText::new(&hub.user.display).strong());
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

            if ui.button("✨ Aris").clicked() {
                app.route = Route::Aris;
            }
        });
    });
}

pub fn show_sidebar(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    let compact = app.sidebar_collapsed;

    let nav_items: &[(&str, &str, Route)] = &[
        ("🏛", "Inicio (Hub)", Route::Hub),
        ("✨", "Aris IA", Route::Aris),
        ("💬", "Mensajería", Route::Chat),
        ("🎨", "Neon Canvas", Route::NeonCanvas),
        ("📄", "Documentos", Route::Placeholder("Documentos")),
        ("🌐", "Comunidades", Route::Placeholder("Comunidades")),
        ("⚙", "Ajustes y Tema", Route::Settings),
    ];

    ui.vertical(|ui| {
        ui.add_space(4.0);
        for (ico, label, target) in nav_items {
            let active = app.route == *target;
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
                app.route = *target;
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            if !compact {
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
                ui.label(egui::RichText::new("Tema rápido").small().color(p.text_muted));
                ui.separator();
            }
        });
    });
}
