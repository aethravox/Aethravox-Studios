use egui::{Color32, Rounding, Stroke};

use crate::app::{AethravoxApp, NeonStroke};
use crate::theme::{card_frame, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    card_frame(p).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("🎨 Pizarra / Neon Canvas");
            ui.separator();

            // Modos de tablero (pensado para aulas PDI y modo neón)
            for (mode, label) in [(0u8, "Neón"), (1, "Pizarra Blanca"), (2, "Pizarra Verde")] {
                if ui
                    .selectable_label(app.neon_board_mode == mode, label)
                    .clicked()
                {
                    app.neon_board_mode = mode;
                    if mode == 1 && app.neon_color == Color32::from_rgb(0, 212, 255) {
                        app.neon_color = Color32::from_rgb(20, 24, 38);
                    }
                }
            }

            ui.separator();

            let swatches = [
                ("Cian", Color32::from_rgb(0, 212, 255)),
                ("Dorado", Color32::from_rgb(224, 163, 8)),
                ("Neón", Color32::from_rgb(147, 51, 234)),
                ("Verde", Color32::from_rgb(0, 255, 136)),
                ("Rojo", Color32::from_rgb(255, 62, 62)),
                ("Blanco", Color32::from_rgb(245, 245, 250)),
                ("Negro", Color32::from_rgb(20, 24, 38)),
            ];
            for (name, c) in swatches {
                let selected = app.neon_color == c;
                let btn = egui::Button::new(egui::RichText::new(name).color(c))
                    .stroke(if selected {
                        Stroke::new(2.0_f32, c)
                    } else {
                        Stroke::new(1.0_f32, p.border_subtle)
                    })
                    .rounding(Rounding::same(6.0));
                if ui.add(btn).clicked() {
                    app.neon_color = c;
                }
            }

            ui.separator();

            // Presets táctiles rápidos (ideales para PDI y móvil)
            for (lbl, w) in [("S", 2.5_f32), ("M", 5.0), ("L", 9.0), ("XL", 14.0)] {
                if ui
                    .selectable_label((app.neon_width - w).abs() < 0.6, lbl)
                    .clicked()
                {
                    app.neon_width = w;
                }
            }
            ui.add(egui::Slider::new(&mut app.neon_width, 1.5..=18.0).text("Grosor"));

            if ui.button("↶ Deshacer").clicked() {
                app.neon_strokes.pop();
                app.neon_active_stroke = None;
            }
            if ui.button("🗑 Limpiar").clicked() {
                app.neon_strokes.clear();
                app.neon_active_stroke = None;
            }
            let fs_lbl = if app.fullscreen { "🗗 Ventana" } else { "⛶ Pantalla PDI" };
            if ui.button(fs_lbl).clicked() {
                app.toggle_fullscreen(&ctx);
            }
        });
    });

    ui.add_space(10.0);

    let avail = ui.available_size();
    let (response, painter) = ui.allocate_painter(avail, egui::Sense::drag());
    let rect = response.rect;

    let board_bg = match app.neon_board_mode {
        1 => Color32::from_rgb(248, 250, 252),
        2 => Color32::from_rgb(16, 54, 38),
        _ => Color32::from_rgb(6, 3, 14),
    };

    painter.rect_filled(rect, Rounding::same(12.0), board_bg);
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0_f32, p.border_strong));

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
            if app.neon_board_mode == 0 {
                // Glow pass en modo neón
                painter.add(egui::Shape::line(
                    s.points.clone(),
                    Stroke::new(s.width * 2.6, s.color.gamma_multiply(0.22)),
                ));
            }
            painter.add(egui::Shape::line(
                s.points.clone(),
                Stroke::new(s.width, s.color),
            ));
        }
    }
}
