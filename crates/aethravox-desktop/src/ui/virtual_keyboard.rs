use egui::{Color32, Rounding};

use crate::app::AethravoxApp;
use crate::theme::Palette;

enum VkKey {
    Text(&'static str),
    Shift,
    Symbols,
    Backspace,
    Space,
    Enter,
    Hide,
}

pub fn show(app: &mut AethravoxApp, ctx: &egui::Context, p: &Palette) {
    let mobile = app.is_mobile_layout(ctx);
    let compact_h = if mobile { 38.0 } else { 34.0 };

    let resp = egui::TopBottomPanel::bottom("virtual_keyboard")
        .frame(
            egui::Frame::none()
                .fill(p.bg_surface)
                .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                .inner_margin(egui::Margin {
                    left: 6.0,
                    right: 6.0,
                    top: 6.0,
                    bottom: if cfg!(any(target_os = "android", target_os = "ios")) {
                        14.0
                    } else {
                        6.0
                    },
                }),
        )
        .show(ctx, |ui| {
            let mut action: Option<VkKey> = None;

            let rows: [&[&str]; 3] = if app.vk_symbols {
                [
                    &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
                    &["@", ".", "_", "-", "/", ":", "?", "!", "+", "="],
                    &["(", ")", "#", "*", "&", "%", "$", ",", ";", "'"],
                ]
            } else if app.vk_shift {
                [
                    &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
                    &["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ñ"],
                    &["Z", "X", "C", "V", "B", "N", "M", "@", ".", "-"],
                ]
            } else {
                [
                    &["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
                    &["a", "s", "d", "f", "g", "h", "j", "k", "l", "ñ"],
                    &["z", "x", "c", "v", "b", "n", "m", "@", ".", "-"],
                ]
            };

            let avail_w = ui.available_width();
            let gap = 3.0_f32;

            for row in rows {
                let n = row.len() as f32;
                let key_w = ((avail_w - gap * (n - 1.0)) / n).max(20.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &ch in row {
                        let btn = egui::Button::new(
                            egui::RichText::new(ch)
                                .size(if mobile { 15.0 } else { 14.0 })
                                .color(p.text_primary),
                        )
                        .fill(p.bg_elevated)
                        .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                        .rounding(Rounding::same(6.0))
                        .min_size(egui::vec2(key_w, compact_h));
                        if ui.add(btn).clicked() {
                            action = Some(VkKey::Text(ch));
                        }
                    }
                });
                ui.add_space(4.0);
            }

            // Bottom control row: Shift / 123 / Space / Backspace / Enter / Hide
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let total = ui.available_width();
                let sym_w = (total * 0.14).clamp(42.0, 68.0);
                let shift_w = (total * 0.12).clamp(36.0, 58.0);
                let back_w = (total * 0.14).clamp(42.0, 68.0);
                let enter_w = (total * 0.16).clamp(48.0, 78.0);
                let hide_w = (total * 0.12).clamp(36.0, 56.0);
                let space_w =
                    (total - sym_w - shift_w - back_w - enter_w - hide_w - gap * 5.0).max(54.0);

                let sym_lbl = if app.vk_symbols { "ABC" } else { "123" };
                if key_ctrl_btn(ui, p, sym_lbl, sym_w, compact_h, app.vk_symbols).clicked() {
                    action = Some(VkKey::Symbols);
                }
                if key_ctrl_btn(ui, p, "⇧", shift_w, compact_h, app.vk_shift).clicked() {
                    action = Some(VkKey::Shift);
                }
                if key_ctrl_btn(ui, p, "Espacio", space_w, compact_h, false).clicked() {
                    action = Some(VkKey::Space);
                }
                if key_ctrl_btn(ui, p, "⌫", back_w, compact_h, false).clicked() {
                    action = Some(VkKey::Backspace);
                }
                let enter_btn = egui::Button::new(
                    egui::RichText::new("↵")
                        .strong()
                        .size(15.0)
                        .color(Color32::from_rgb(12, 10, 18)),
                )
                .fill(p.accent_primary)
                .rounding(Rounding::same(6.0))
                .min_size(egui::vec2(enter_w, compact_h));
                if ui.add(enter_btn).clicked() {
                    action = Some(VkKey::Enter);
                }
                if key_ctrl_btn(ui, p, "✕", hide_w, compact_h, false).clicked() {
                    action = Some(VkKey::Hide);
                }
            });

            if let Some(act) = action {
                match act {
                    VkKey::Text(t) => {
                        restore_focus(app, ctx);
                        app.vk_pending_events.push(egui::Event::Text(t.to_string()));
                        if app.vk_shift && !app.vk_symbols {
                            app.vk_shift = false;
                        }
                    }
                    VkKey::Space => {
                        restore_focus(app, ctx);
                        app.vk_pending_events.push(egui::Event::Text(" ".to_string()));
                    }
                    VkKey::Backspace => {
                        restore_focus(app, ctx);
                        app.vk_pending_events.push(egui::Event::Key {
                            key: egui::Key::Backspace,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                    VkKey::Enter => {
                        restore_focus(app, ctx);
                        app.vk_pending_events.push(egui::Event::Key {
                            key: egui::Key::Enter,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                    VkKey::Shift => {
                        restore_focus(app, ctx);
                        app.vk_shift = !app.vk_shift;
                    }
                    VkKey::Symbols => {
                        restore_focus(app, ctx);
                        app.vk_symbols = !app.vk_symbols;
                    }
                    VkKey::Hide => {
                        app.vk_manual_open = false;
                        app.vk_dismissed = true;
                        #[cfg(target_os = "android")]
                        if let Some(android_app) = &app.android_app {
                            android_app.hide_soft_input(true);
                        }
                    }
                }
                ctx.request_repaint();
            } else if ctx.input(|i| i.pointer.any_down() || i.pointer.any_released()) {
                if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    if ui.min_rect().contains(pos) {
                        restore_focus(app, ctx);
                    }
                }
            }
        });

    app.vk_panel_rect = Some(resp.response.rect);
}

fn restore_focus(app: &AethravoxApp, ctx: &egui::Context) {
    if let Some(fid) = app.vk_last_focus {
        ctx.memory_mut(|m| m.request_focus(fid));
    }
}

fn key_ctrl_btn(
    ui: &mut egui::Ui,
    p: &Palette,
    label: &str,
    width: f32,
    height: f32,
    active: bool,
) -> egui::Response {
    let fill = if active {
        p.accent_secondary.gamma_multiply(0.28)
    } else {
        p.bg_elevated
    };
    let stroke_col = if active {
        p.accent_secondary
    } else {
        p.border_subtle
    };
    let btn = egui::Button::new(egui::RichText::new(label).small().color(p.text_primary))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0_f32, stroke_col))
        .rounding(Rounding::same(6.0))
        .min_size(egui::vec2(width, height));
    ui.add(btn)
}
