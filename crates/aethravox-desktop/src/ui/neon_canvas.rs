use egui::{Color32, Rounding, Stroke};

use crate::app::{AethravoxApp, NeonStroke};
use crate::theme::{card_frame, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    card_frame(p).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("🎨 Neon Canvas (Nativo GPU)");
            ui.separator();

            let swatches = [
                ("Cian", Color32::from_rgb(0, 212, 255)),
                ("Dorado", Color32::from_rgb(224, 163, 8)),
                ("Neón", Color32::from_rgb(147, 51, 234)),
                ("Verde", Color32::from_rgb(0, 255, 136)),
                ("Rojo", Color32::from_rgb(255, 62, 62)),
            ];
            for (name, c) in swatches {
                let selected = app.neon_color == c;
                let btn = egui::Button::new(egui::RichText::new(name).color(c))
                    .stroke(if selected {
                        Stroke::new(1.5, c)
                    } else {
                        Stroke::new(1.0, p.border_subtle)
                    })
                    .rounding(Rounding::same(6.0));
                if ui.add(btn).clicked() {
                    app.neon_color = c;
                }
            }

            ui.add(egui::Slider::new(&mut app.neon_width, 1.5..=14.0).text("Grosor"));

            if ui.button("Limpiar").clicked() {
                app.neon_strokes.clear();
                app.neon_active_stroke = None;
            }
        });
    });

    ui.add_space(10.0);

    let avail = ui.available_size();
    let (response, painter) = ui.allocate_painter(avail, egui::Sense::drag());
    let rect = response.rect;

    painter.rect_filled(rect, Rounding::same(12.0), Color32::from_rgb(6, 3, 14));
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, p.border_strong));

    if let Some(pos) = response.interact_pointer_pos() {
        if rect.contains(pos) {
            let stroke = app.neon_active_stroke.get_or_insert_with(|| NeonStroke {
                points: Vec::new(),
                color: app.neon_color,
                width: app.neon_width,
            });
            if stroke.points.last().map_or(true, |last| last.distance(pos) > 2.0) {
                stroke.points.push(pos);
            }
        }
    } else if let Some(finished) = app.neon_active_stroke.take() {
        if !finished.points.is_empty() {
            app.neon_strokes.push(finished);
        }
    }

    for s in app.neon_strokes.iter().chain(app.neon_active_stroke.iter()) {
        if s.points.len() >= 2 {
            // Glow pass + crisp core pass
            painter.add(egui::Shape::line(
                s.points.clone(),
                Stroke::new(s.width * 2.6, s.color.gamma_multiply(0.22)),
            ));
            painter.add(egui::Shape::line(
                s.points.clone(),
                Stroke::new(s.width, s.color),
            ));
        }
    }
}
