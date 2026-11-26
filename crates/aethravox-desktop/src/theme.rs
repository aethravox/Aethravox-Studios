use aethravox_core::{ColorMode, ThemeKind};
use egui::{Color32, FontId, Rounding, Stroke, Style, TextStyle, Visuals};

/// Paleta completa de Aethravox Studios traducida directamente desde
/// `aethravox.css`, `shadow.css` y `commoncents.css` (modo oscuro y claro).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_base: Color32,
    pub bg_surface: Color32,
    pub bg_elevated: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub accent_primary: Color32,
    pub accent_primary_hover: Color32,
    pub accent_secondary: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub success: Color32,
    pub danger: Color32,
    pub warning: Color32,
}

impl Palette {
    pub fn from(theme: ThemeKind, mode: ColorMode) -> Self {
        match (theme, mode) {
            (ThemeKind::Aethravox, ColorMode::Dark) => Self {
                bg_base: Color32::from_rgb(6, 3, 12),          // #06030C
                bg_surface: Color32::from_rgb(13, 8, 24),      // #0D0818
                bg_elevated: Color32::from_rgb(22, 16, 31),    // #16101F
                border_subtle: Color32::from_rgb(30, 24, 48),  // #1E1830
                border_strong: Color32::from_rgb(55, 43, 84),
                accent_primary: Color32::from_rgb(224, 163, 8),    // #e0a308
                accent_primary_hover: Color32::from_rgb(245, 197, 66), // #f5c542
                accent_secondary: Color32::from_rgb(0, 212, 255),  // #00d4ff
                text_primary: Color32::from_rgb(248, 248, 252),
                text_muted: Color32::from_rgb(154, 152, 170),
                success: Color32::from_rgb(0, 255, 136),       // #00ff88
                danger: Color32::from_rgb(255, 62, 62),        // #ff3e3e
                warning: Color32::from_rgb(255, 140, 66),
            },
            (ThemeKind::Shadow, ColorMode::Dark) => Self {
                bg_base: Color32::from_rgb(6, 3, 12),
                bg_surface: Color32::from_rgb(13, 8, 24),
                bg_elevated: Color32::from_rgb(22, 16, 31),
                border_subtle: Color32::from_rgb(46, 16, 101), // #2E1065
                border_strong: Color32::from_rgb(88, 28, 135),
                accent_primary: Color32::from_rgb(147, 51, 234),   // #9333EA
                accent_primary_hover: Color32::from_rgb(168, 85, 247),
                accent_secondary: Color32::from_rgb(216, 180, 254), // #D8B4FE
                text_primary: Color32::from_rgb(250, 245, 255),
                text_muted: Color32::from_rgb(156, 163, 175),
                success: Color32::from_rgb(0, 255, 136),
                danger: Color32::from_rgb(255, 62, 62),
                warning: Color32::from_rgb(255, 140, 66),
            },
            (ThemeKind::CommonCents, ColorMode::Dark) => Self {
                bg_base: Color32::from_rgb(6, 3, 12),
                bg_surface: Color32::from_rgb(13, 8, 24),
                bg_elevated: Color32::from_rgb(22, 16, 31),
                border_subtle: Color32::from_rgb(30, 41, 59),  // #1E293B
                border_strong: Color32::from_rgb(51, 65, 85),
                accent_primary: Color32::from_rgb(34, 197, 94),    // #22c55e
                accent_primary_hover: Color32::from_rgb(74, 222, 128),
                accent_secondary: Color32::from_rgb(59, 130, 246), // #3b82f6
                text_primary: Color32::from_rgb(248, 250, 252),
                text_muted: Color32::from_rgb(148, 163, 184),
                success: Color32::from_rgb(34, 197, 94),
                danger: Color32::from_rgb(239, 68, 68),
                warning: Color32::from_rgb(234, 179, 8),
            },
            (ThemeKind::Aethravox, ColorMode::Light) => Self {
                bg_base: Color32::from_rgb(246, 244, 250),
                bg_surface: Color32::from_rgb(255, 255, 255),
                bg_elevated: Color32::from_rgb(238, 234, 246),
                border_subtle: Color32::from_rgb(222, 216, 235),
                border_strong: Color32::from_rgb(190, 180, 212),
                accent_primary: Color32::from_rgb(194, 134, 0),
                accent_primary_hover: Color32::from_rgb(224, 163, 8),
                accent_secondary: Color32::from_rgb(0, 148, 190),
                text_primary: Color32::from_rgb(20, 16, 32),
                text_muted: Color32::from_rgb(98, 94, 118),
                success: Color32::from_rgb(16, 163, 94),
                danger: Color32::from_rgb(220, 38, 38),
                warning: Color32::from_rgb(217, 119, 6),
            },
            (ThemeKind::Shadow, ColorMode::Light) => Self {
                bg_base: Color32::from_rgb(247, 244, 253),
                bg_surface: Color32::from_rgb(255, 255, 255),
                bg_elevated: Color32::from_rgb(240, 233, 252),
                border_subtle: Color32::from_rgb(224, 208, 248),
                border_strong: Color32::from_rgb(192, 158, 242),
                accent_primary: Color32::from_rgb(126, 34, 206),
                accent_primary_hover: Color32::from_rgb(147, 51, 234),
                accent_secondary: Color32::from_rgb(107, 33, 168),
                text_primary: Color32::from_rgb(22, 12, 38),
                text_muted: Color32::from_rgb(100, 92, 122),
                success: Color32::from_rgb(16, 163, 94),
                danger: Color32::from_rgb(220, 38, 38),
                warning: Color32::from_rgb(217, 119, 6),
            },
            (ThemeKind::CommonCents, ColorMode::Light) => Self {
                bg_base: Color32::from_rgb(244, 248, 250),
                bg_surface: Color32::from_rgb(255, 255, 255),
                bg_elevated: Color32::from_rgb(233, 241, 247),
                border_subtle: Color32::from_rgb(210, 224, 235),
                border_strong: Color32::from_rgb(176, 196, 214),
                accent_primary: Color32::from_rgb(22, 163, 74),
                accent_primary_hover: Color32::from_rgb(34, 197, 94),
                accent_secondary: Color32::from_rgb(37, 99, 235),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_muted: Color32::from_rgb(88, 102, 124),
                success: Color32::from_rgb(22, 163, 74),
                danger: Color32::from_rgb(220, 38, 38),
                warning: Color32::from_rgb(202, 138, 4),
            },
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: ThemeKind, mode: ColorMode) -> Palette {
    let p = Palette::from(theme, mode);

    let mut style = Style::default();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.interact_size = egui::vec2(38.0, 32.0);

    style.text_styles.insert(TextStyle::Heading, FontId::proportional(22.0));
    style.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    style.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.5));

    let mut visuals = match mode {
        ColorMode::Dark => Visuals::dark(),
        ColorMode::Light => Visuals::light(),
    };

    visuals.panel_fill = p.bg_base;
    visuals.window_fill = p.bg_surface;
    visuals.extreme_bg_color = p.bg_elevated;
    visuals.faint_bg_color = p.bg_elevated;
    visuals.override_text_color = Some(p.text_primary);
    visuals.window_rounding = Rounding::same(12.0);
    visuals.window_stroke = Stroke::new(1.0, p.border_subtle);
    visuals.selection.bg_fill = p.accent_primary.gamma_multiply(0.28);
    visuals.selection.stroke = Stroke::new(1.0, p.accent_primary);

    visuals.widgets.noninteractive.bg_fill = p.bg_surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = p.bg_elevated;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);

    visuals.widgets.hovered.bg_fill = p.bg_elevated;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2, p.accent_primary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2, p.text_primary);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);

    visuals.widgets.active.bg_fill = p.accent_primary.gamma_multiply(0.22);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, p.accent_primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, p.text_primary);
    visuals.widgets.active.rounding = Rounding::same(8.0);

    style.visuals = visuals;
    ctx.set_style(style);
    p
}

pub fn card_frame(p: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(16.0))
}

pub fn elevated_frame(p: &Palette) -> egui::Frame {
    egui::Frame::none()
        .fill(p.bg_elevated)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(12.0))
}

pub fn parse_hex_color(hex: &str, fallback: Color32) -> Color32 {
    let s = hex.trim().trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    fallback
}

pub fn primary_button(ui: &mut egui::Ui, p: &Palette, label: &str) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .strong()
            .color(Color32::from_rgb(12, 10, 18)),
    )
    .fill(p.accent_primary)
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(8.0));
    ui.add(btn)
}

pub fn badge_pill(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().strong().color(fg));
        });
}
