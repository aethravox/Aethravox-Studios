use aethravox_core::{AexosLinkRequest, AexosSavePrefsRequest, AexosSwitchRequest};
use egui::{Color32, Rounding};

use crate::api::{
    spawn_aexos_delete, spawn_aexos_link, spawn_aexos_profiles, spawn_aexos_save_prefs,
    spawn_aexos_switch,
};
use crate::app::{AethravoxApp, Route};
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut save_prefs = false;
    let mut nav_target: Option<Route> = None;
    let mut switch_req: Option<AexosSwitchRequest> = None;
    let mut delete_profile_id: Option<i64> = None;
    let mut link_req: Option<AexosLinkRequest> = None;

    let active_idx = app
        .aexos_profiles
        .iter()
        .position(|pr| pr.user_id == app.aexos_active_user_id)
        .or_else(|| if app.aexos_profiles.is_empty() { None } else { Some(0) });

    if let Some(idx) = active_idx {
        let wp = app.aexos_profiles[idx].wallpaper.clone();
        let bg_tint = match wp.as_str() {
            "obsidian" => Color32::from_rgb(8, 8, 14),
            "aurora" => Color32::from_rgb(8, 22, 26),
            _ => Color32::from_rgb(16, 10, 32),
        };

        let mobile = app.is_mobile_layout(&ctx);

        // Top taskbar de aexOS
        card_frame(p).fill(bg_tint).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("🖥 aexOS Desktop");
                let cur = &app.aexos_profiles[idx];
                badge_pill(
                    ui,
                    &format!("{} ({})", cur.display_name, cur.role_display),
                    p.accent_primary.gamma_multiply(0.18),
                    p.accent_primary,
                );
                if primary_button(ui, p, "💾 Guardar entorno").clicked() {
                    save_prefs = true;
                }
                if ui.button("+ Vincular cuenta").clicked() {
                    app.aexos_link_open = !app.aexos_link_open;
                }
                if ui.button("↻ Actualizar").clicked() {
                    refresh = true;
                }
            });
            if let Some(err) = &app.aexos_error {
                ui.label(egui::RichText::new(err).small().color(p.danger));
            }
        });

        ui.add_space(10.0);

        if app.aexos_link_open {
            elevated_frame(p).show(ui, |ui| {
                ui.heading("Vincular cuenta existente a aexOS");
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.aexos_link_user)
                            .hint_text("Usuario"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.aexos_link_pass)
                            .password(true)
                            .hint_text("Contraseña"),
                    );
                    if primary_button(ui, p, "Vincular").clicked()
                        && !app.aexos_link_user.trim().is_empty()
                    {
                        link_req = Some(AexosLinkRequest {
                            username: app.aexos_link_user.trim().to_string(),
                            password: app.aexos_link_pass.clone(),
                        });
                        app.aexos_link_user.clear();
                        app.aexos_link_pass.clear();
                        app.aexos_link_open = false;
                    }
                });
            });
            ui.add_space(8.0);
        }

        if let Some(target_id) = app.aexos_switch_target {
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Contraseña del perfil seleccionado:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.aexos_switch_pass)
                            .password(true)
                            .hint_text("Contraseña…"),
                    );
                    if primary_button(ui, p, "Cambiar").clicked() {
                        switch_req = Some(AexosSwitchRequest {
                            user_id: target_id,
                            password: Some(app.aexos_switch_pass.clone()),
                        });
                        app.aexos_switch_pass.clear();
                        app.aexos_switch_target = None;
                    }
                    if ui.button("Cancelar").clicked() {
                        app.aexos_switch_target = None;
                        app.aexos_switch_pass.clear();
                    }
                });
            });
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical()
            .id_salt("aexos_scroll")
            .show(ui, |ui| {
                if mobile {
                    show_aexos_left(
                        app,
                        ui,
                        p,
                        idx,
                        bg_tint,
                        &mut nav_target,
                        &mut delete_profile_id,
                        &mut switch_req,
                    );
                    ui.add_space(10.0);
                    show_aexos_right(app, ui, p, idx);
                } else {
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            show_aexos_left(
                                app,
                                ui,
                                p,
                                idx,
                                bg_tint,
                                &mut nav_target,
                                &mut delete_profile_id,
                                &mut switch_req,
                            );
                        });
                        cols[1].vertical(|ui| {
                            show_aexos_right(app, ui, p, idx);
                        });
                    });
                }
            });
    } else {
        card_frame(p).show(ui, |ui| {
            ui.heading("🖥 aexOS Desktop");
            ui.label("Inicia sesión para cargar tus perfiles de aexOS.");
        });
    }

    if refresh && !app.offline_mode && app.token.is_some() {
        spawn_aexos_profiles(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if save_prefs {
        if let Some(idx) = active_idx {
            let cur = app.aexos_profiles[idx].clone();
            if !app.offline_mode && app.token.is_some() {
                spawn_aexos_save_prefs(
                    &app.rt,
                    app.tx.clone(),
                    ctx.clone(),
                    app.server_url.clone(),
                    app.token.clone(),
                    AexosSavePrefsRequest {
                        profile_user_id: cur.user_id,
                        wallpaper: cur.wallpaper,
                        show_desktop_icons: cur.show_desktop_icons,
                        pinned_apps_json: cur.pinned_apps_json,
                        files_json: cur.files_json,
                        notes_text: cur.notes_text,
                    },
                );
            } else {
                app.status_banner = Some("Preferencias de aexOS guardadas en sesión local.".into());
            }
        }
    }
    if let Some(req) = link_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_aexos_link(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(req) = switch_req {
        if !app.offline_mode && app.token.is_some() {
            spawn_aexos_switch(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(pid) = delete_profile_id {
        if !app.offline_mode && app.token.is_some() {
            spawn_aexos_delete(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                pid,
            );
        }
    }
    if let Some(target) = nav_target {
        app.navigate_to(&ctx, target);
    }
}

#[allow(clippy::too_many_arguments)]
fn show_aexos_left(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    idx: usize,
    bg_tint: Color32,
    nav_target: &mut Option<Route>,
    delete_profile_id: &mut Option<i64>,
    switch_req: &mut Option<AexosSwitchRequest>,
) {
    card_frame(p).fill(bg_tint).show(ui, |ui| {
        ui.heading("Aplicaciones del escritorio");
        ui.add_space(6.0);
        let cur = &mut app.aexos_profiles[idx];
        ui.horizontal_wrapped(|ui| {
            ui.label("Fondo:");
            for (k, label) in [
                ("nebula", "Nebula"),
                ("obsidian", "Obsidian"),
                ("aurora", "Aurora"),
            ] {
                if ui.selectable_label(cur.wallpaper == k, label).clicked() {
                    cur.wallpaper = k.to_string();
                }
            }
        });
        ui.checkbox(&mut cur.show_desktop_icons, "Mostrar iconos en escritorio");
        ui.separator();

        if cur.show_desktop_icons {
            let tiles: &[(&str, &str, Route)] = &[
                ("✨", "Aris IA", Route::Aris),
                ("💬", "Mensajería", Route::Chat),
                ("🎨", "Neon Canvas", Route::NeonCanvas),
                ("📄", "Documentos", Route::Docs),
                ("📺", "aexTube", Route::AexTube),
                ("🧪", "Playground", Route::Playground),
                ("👥", "Amigos", Route::Friends),
                ("⚙", "Ajustes", Route::Settings),
            ];
            let grid_cols = if ui.available_width() < 340.0 { 1 } else { 2 };
            egui::Grid::new("aexos_apps_grid")
                .num_columns(grid_cols)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (i, (ico, title, target)) in tiles.iter().enumerate() {
                        let btn = egui::Button::new(format!("{ico}  {title}"))
                            .rounding(Rounding::same(8.0))
                            .min_size(egui::vec2(145.0, 38.0));
                        if ui.add(btn).clicked() {
                            *nav_target = Some(*target);
                        }
                        if (i + 1) % grid_cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
    });

    ui.add_space(10.0);
    card_frame(p).show(ui, |ui| {
        ui.heading("Perfiles vinculados");
        ui.separator();
        for pr in &app.aexos_profiles {
            elevated_frame(p).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(&pr.display_name).strong());
                    ui.label(
                        egui::RichText::new(format!("@{}", pr.username))
                            .small()
                            .color(p.text_muted),
                    );
                    if pr.user_id == app.aexos_active_user_id {
                        badge_pill(
                            ui,
                            "Activo",
                            p.success.gamma_multiply(0.18),
                            p.success,
                        );
                    }
                    if pr.user_id != app.aexos_active_user_id && ui.button("Usar").clicked() {
                        if pr.is_primary {
                            *switch_req = Some(AexosSwitchRequest {
                                user_id: pr.user_id,
                                password: None,
                            });
                        } else {
                            app.aexos_switch_target = Some(pr.user_id);
                        }
                    }
                    if !pr.is_primary
                        && ui
                            .button(egui::RichText::new("Quitar").color(p.danger))
                            .clicked()
                    {
                        *delete_profile_id = Some(pr.user_id);
                    }
                });
            });
            ui.add_space(5.0);
        }
    });
}

fn show_aexos_right(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette, idx: usize) {
    let cur = &mut app.aexos_profiles[idx];
    card_frame(p).show(ui, |ui| {
        ui.heading("📝 Notas de aexOS");
        ui.add(
            egui::TextEdit::multiline(&mut cur.notes_text)
                .desired_width(f32::INFINITY)
                .desired_rows(10)
                .hint_text("Tus notas guardadas en la nube de aexOS…"),
        );
    });
    ui.add_space(10.0);
    card_frame(p).show(ui, |ui| {
        ui.heading("📁 Archivos virtuales");
        let mut files: Vec<String> = serde_json::from_str(&cur.files_json).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.aexos_new_file_name)
                    .desired_width((ui.available_width() - 95.0).max(120.0))
                    .hint_text("Nombre de archivo o nota…"),
            );
            if ui.button("+ Añadir").clicked() && !app.aexos_new_file_name.trim().is_empty() {
                files.push(app.aexos_new_file_name.trim().to_string());
                app.aexos_new_file_name.clear();
                cur.files_json = serde_json::to_string(&files).unwrap_or_else(|_| "[]".into());
            }
        });
        ui.add_space(6.0);
        if files.is_empty() {
            ui.label(
                egui::RichText::new("Sin archivos en este perfil.")
                    .small()
                    .color(p.text_muted),
            );
        } else {
            let mut remove_idx: Option<usize> = None;
            for (i, f) in files.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("📄 {f}"));
                    if ui.small_button("✕").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(ri) = remove_idx {
                files.remove(ri);
                cur.files_json = serde_json::to_string(&files).unwrap_or_else(|_| "[]".into());
            }
        }
    });
}
