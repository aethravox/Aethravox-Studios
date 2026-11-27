use crate::api::spawn_fetch_downloads;
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;

    egui::ScrollArea::vertical()
        .id_salt("downloads_scroll")
        .show(ui, |ui| {
            card_frame(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("⬇ Centro de Descargas");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("↻ Actualizar").clicked() {
                            refresh = true;
                        }
                    });
                });
                ui.label(
                    egui::RichText::new("Clientes nativos, paquetes oficiales y releases de Aethravox Studios.")
                        .color(p.text_muted),
                );
            });

            if let Some(err) = &app.downloads_error {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(err).small().color(p.danger));
            }

            ui.add_space(10.0);
            let base = app.server_url.trim_end_matches('/').to_string();

            for plat in &app.downloads {
                card_frame(p).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(format!("{}  {}", plat.icon, plat.name));
                        if plat.coming_soon {
                            badge_pill(
                                ui,
                                "Próximamente",
                                p.accent_primary.gamma_multiply(0.18),
                                p.accent_primary,
                            );
                        }
                    });
                    ui.separator();

                    if plat.apps.is_empty() {
                        ui.label(
                            egui::RichText::new("Sin builds publicados para esta plataforma todavía.")
                                .small()
                                .color(p.text_muted),
                        );
                    } else {
                        for app_entry in &plat.apps {
                            ui.label(egui::RichText::new(&app_entry.name).strong().size(15.0));
                            ui.add_space(4.0);
                            for f in &app_entry.files {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&f.orig_name).strong(),
                                                );
                                                badge_pill(
                                                    ui,
                                                    &format!("v{}", f.version),
                                                    p.accent_secondary.gamma_multiply(0.16),
                                                    p.accent_secondary,
                                                );
                                            });
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} · {}",
                                                    f.size_mb, f.created_at
                                                ))
                                                .small()
                                                .color(p.text_muted),
                                            );
                                        });
                                        if primary_button(ui, p, "Descargar").clicked() {
                                            let full = if f.url.starts_with("http") {
                                                f.url.clone()
                                            } else {
                                                format!("{base}{}", f.url)
                                            };
                                            ui.ctx().open_url(egui::OpenUrl::new_tab(full));
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                            ui.add_space(6.0);
                        }
                    }
                });
                ui.add_space(10.0);
            }
        });

    if refresh && !app.offline_mode {
        spawn_fetch_downloads(
            &app.rt,
            app.tx.clone(),
            ctx,
            app.server_url.clone(),
            app.token.clone(),
        );
    }
}
