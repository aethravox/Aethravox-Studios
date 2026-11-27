use aethravox_core::{ColorMode, ThemeKind};
use egui::{Color32, Rounding, Stroke};

use crate::app::AethravoxApp;
use crate::theme::{card_frame, elevated_frame, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, ctx: &egui::Context, p: &Palette) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        card_frame(p).show(ui, |ui| {
            ui.heading("Tema visual y modo de color");
            ui.label(
                egui::RichText::new("Elige el estilo y la luminosidad de tu plataforma.")
                    .color(p.text_muted),
            );
            ui.add_space(12.0);

            ui.horizontal(|ui| {
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

            ui.columns(3, |cols| {
                for (idx, (kind, subtitle, c1, c2)) in themes.into_iter().enumerate() {
                    let active = app.theme == kind;
                    cols[idx].vertical(|ui| {
                        let resp = elevated_frame(p)
                            .stroke(Stroke::new(
                                if active { 2.0_f32 } else { 1.0_f32 },
                                if active { p.accent_primary } else { p.border_subtle },
                            ))
                            .show(ui, |ui| {
                                ui.set_min_height(110.0);
                                ui.horizontal(|ui| {
                                    let (r1, painter) = ui.allocate_painter(
                                        egui::vec2(32.0, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(r1.rect.min, egui::vec2(14.0, 14.0)),
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
                                ui.label(egui::RichText::new(kind.display_name()).strong());
                                ui.label(egui::RichText::new(subtitle).small().color(p.text_muted));
                            })
                            .response
                            .interact(egui::Sense::click());

                        if resp.clicked() {
                            app.set_theme(ctx, kind);
                        }
                    });
                }
            });
        });

        ui.add_space(14.0);

        if let Some(hub) = &app.hub {
            card_frame(p).show(ui, |ui| {
                ui.heading("Cuenta");
                ui.add_space(6.0);
                ui.label(format!("Usuario: {}", hub.user.username));
                ui.label(format!("Rol: {}", hub.user.role_display));
                ui.label(format!("Servidor: {}", app.server_url));
            });
        }
    });
}
