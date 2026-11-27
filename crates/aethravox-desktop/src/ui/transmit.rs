use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let base = app.server_url.trim_end_matches('/').to_string();

    egui::ScrollArea::vertical()
        .id_salt("transmit_scroll")
        .show(ui, |ui| {
            card_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("📡 Transmitir — Pantalla P2P");
                    badge_pill(
                        ui,
                        "WebRTC Peer-to-Peer",
                        p.accent_secondary.gamma_multiply(0.18),
                        p.accent_secondary,
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Comparte pantalla y audio en tiempo real mediante salas WebRTC señalizadas por el servidor Axum.",
                    )
                    .color(p.text_muted),
                );
                ui.add_space(12.0);

                elevated_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Código / Sala:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut app.transmit_room)
                                .desired_width(ui.available_width().min(220.0))
                                .hint_text("ej. estudio-01"),
                        );
                    });
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        if primary_button(ui, p, "📺 Emitir pantalla").clicked() {
                            let url = if app.transmit_room.trim().is_empty() {
                                format!("{base}/transmitir/enviar")
                            } else {
                                format!(
                                    "{base}/transmitir/enviar?room={}",
                                    app.transmit_room.trim()
                                )
                            };
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                        if ui.button("👁 Ver transmisión").clicked() {
                            let url = if app.transmit_room.trim().is_empty() {
                                format!("{base}/transmitir/recibir")
                            } else {
                                format!(
                                    "{base}/transmitir/recibir?room={}",
                                    app.transmit_room.trim()
                                )
                            };
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                        if ui.button("Abrir selector /transmitir").clicked() {
                            ui.ctx()
                                .open_url(egui::OpenUrl::new_tab(format!("{base}/transmitir")));
                        }
                    });
                });
            });
        });
}
