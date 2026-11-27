use crate::api::spawn_fetch_docs;
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;

    egui::ScrollArea::vertical().id_salt("docs_scroll").show(ui, |ui| {
        card_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("📄 Documentos de Aethravox");
                ui.add(
                    egui::TextEdit::singleline(&mut app.docs_search)
                        .desired_width(ui.available_width().min(240.0))
                        .hint_text("Buscar documento…"),
                );
                if ui.button("↻ Actualizar").clicked() {
                    refresh = true;
                }
            });
        });

        if let Some(err) = &app.docs_error {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(err).small().color(p.danger));
        }

        ui.add_space(10.0);
        let q = app.docs_search.trim().to_lowercase();
        let filtered: Vec<_> = app
            .docs
            .iter()
            .filter(|d| {
                q.is_empty()
                    || d.title.to_lowercase().contains(&q)
                    || d.description.to_lowercase().contains(&q)
                    || d.file_name.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();

        if filtered.is_empty() {
            card_frame(p).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("No hay documentos que coincidan con tu búsqueda.")
                        .color(p.text_muted),
                );
            });
            return;
        }

        let base = app.server_url.trim_end_matches('/').to_string();
        for d in filtered {
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&d.icon).size(26.0));
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(&d.title).strong().size(15.5));
                            badge_pill(
                                ui,
                                &d.file_type.to_uppercase(),
                                p.accent_secondary.gamma_multiply(0.16),
                                p.accent_secondary,
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} · {} · {} descargas",
                                    d.size, d.created_at, d.downloaded
                                ))
                                .small()
                                .color(p.text_muted),
                            );
                        });
                        if !d.description.is_empty() {
                            ui.label(egui::RichText::new(&d.description).small().color(p.text_muted));
                        }
                        ui.label(
                            egui::RichText::new(&d.file_name)
                                .small()
                                .color(p.text_muted),
                        );
                    });
                    if d.has_preview && ui.button("👁 Vista previa").clicked() {
                        let url = format!("{base}/docs/{}/raw", d.id);
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                    if primary_button(ui, p, "⬇ Descargar").clicked() {
                        let url = format!("{base}/docs/{}/download", d.id);
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                });
            });
            ui.add_space(6.0);
        }
    });

    if refresh && !app.offline_mode {
        spawn_fetch_docs(
            &app.rt,
            app.tx.clone(),
            ctx,
            app.server_url.clone(),
            app.token.clone(),
        );
    }
}
