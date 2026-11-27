use aethravox_core::WebAppDto;
use egui::Rounding;

use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, parse_hex_color, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let Some(hub) = app.hub.clone() else {
        ui.centered_and_justified(|ui| {
            ui.label("Cargando hub…");
        });
        return;
    };

    let mobile = app.is_mobile_layout(ui.ctx());

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Hero summary bar
        card_frame(p).show(ui, |ui| {
            if mobile {
                ui.heading(format!("Hola, {}", hub.user.display));
                ui.label(
                    egui::RichText::new("Estudio nativo de Aethravox Studios")
                        .small()
                        .color(p.text_muted),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    stat_chip(ui, p, "Mensajes", hub.stats.chat_unread);
                    stat_chip(ui, p, "En línea", hub.stats.online_count);
                    stat_chip(ui, p, "Docs", hub.stats.doc_count);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(format!("Hola, {}", hub.user.display));
                        ui.label(
                            egui::RichText::new("Bienvenido al estudio nativo de Aethravox Studios")
                                .color(p.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        stat_chip(ui, p, "Docs", hub.stats.doc_count);
                        stat_chip(ui, p, "En línea", hub.stats.online_count);
                        stat_chip(ui, p, "Mensajes", hub.stats.chat_unread);
                    });
                });
            }

            ui.add_space(12.0);

            // Search + category filter bar
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.hub_search)
                        .desired_width(ui.available_width().min(260.0))
                        .hint_text("Buscar aplicación o herramienta…"),
                );

                let all_active = app.hub_cat_filter == "all";
                if ui.selectable_label(all_active, "Todas").clicked() {
                    app.hub_cat_filter = "all".to_string();
                }
                for cat in &hub.categories {
                    let active = app.hub_cat_filter == cat.slug;
                    if ui
                        .selectable_label(active, format!("{} ({})", cat.name, cat.items.len()))
                        .clicked()
                    {
                        app.hub_cat_filter = cat.slug.clone();
                    }
                }
            });
        });

        ui.add_space(16.0);

        let q = app.hub_search.trim().to_ascii_lowercase();
        let mut clicked_app: Option<WebAppDto> = None;

        for cat in &hub.categories {
            if app.hub_cat_filter != "all" && app.hub_cat_filter != cat.slug {
                continue;
            }

            let filtered: Vec<&WebAppDto> = cat
                .items
                .iter()
                .filter(|w| {
                    if q.is_empty() {
                        true
                    } else {
                        w.name.to_ascii_lowercase().contains(&q)
                            || w.description.to_ascii_lowercase().contains(&q)
                            || w.slug.to_ascii_lowercase().contains(&q)
                    }
                })
                .collect();

            if filtered.is_empty() {
                continue;
            }

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&cat.name).size(18.0).strong());
            });
            ui.add_space(6.0);

            let avail = ui.available_width();
            let cols = ((avail / 270.0).floor() as usize).clamp(1, 4);
            let card_w = (avail - (cols as f32 - 1.0) * 12.0) / cols as f32;

            egui::Grid::new(format!("hub_grid_{}", cat.slug))
                .spacing([12.0, 12.0])
                .show(ui, |ui| {
                    for (idx, w) in filtered.iter().enumerate() {
                        let resp = elevated_frame(p)
                            .show(ui, |ui| {
                                ui.set_width(card_w - 24.0);
                                ui.set_min_height(96.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&w.icon).size(22.0));
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&w.name).strong());
                                            if !w.tag_label.is_empty() && w.tag_label != "Activo" {
                                                let tc = parse_hex_color(&w.tag_color, p.accent_secondary);
                                                badge_pill(
                                                    ui,
                                                    &w.tag_label,
                                                    tc.gamma_multiply(0.22),
                                                    tc,
                                                );
                                            }
                                        });
                                        ui.label(
                                            egui::RichText::new(&w.description)
                                                .small()
                                                .color(p.text_muted),
                                        );
                                    });
                                });
                            })
                            .response
                            .interact(egui::Sense::click());

                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            clicked_app = Some((*w).clone());
                        }

                        if (idx + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(16.0);
        }

        if let Some(w) = clicked_app {
            let ctx = ui.ctx().clone();
            app.open_app(&ctx, &w);
        }
    });
}

fn stat_chip(ui: &mut egui::Ui, p: &Palette, label: &str, value: i64) {
    egui::Frame::none()
        .fill(p.bg_elevated)
        .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(value.to_string())
                        .strong()
                        .color(p.accent_primary),
                );
                ui.label(egui::RichText::new(label).small().color(p.text_muted));
            });
        });
}
