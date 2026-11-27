use aethravox_core::{ZLibFileInput, ZLibPublishRequest};
use egui::{Color32, Rounding};

use crate::api::{
    spawn_create_zlib_token, spawn_fetch_zlib_detail, spawn_fetch_zlibs_mine,
    spawn_fetch_zlibs_packages, spawn_publish_zlib, spawn_revoke_zlib_token,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let has_tok = app.token.is_some() && !app.offline_mode;
    let is_guest = app
        .hub
        .as_ref()
        .map(|h| h.user.is_guest)
        .unwrap_or(!has_tok);

    let mut do_search = false;
    let mut fetch_detail: Option<String> = None;
    let mut fetch_mine = false;
    let mut create_token = false;
    let mut revoke_token_id: Option<i64> = None;
    let mut publish_now = false;

    if app.zlibs_tab == 2 {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let name = if !f.name.is_empty() {
                f.name.clone()
            } else if let Some(path) = &f.path {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Main.zen")
                    .to_string()
            } else {
                continue;
            };
            if let Some(bytes) = f.bytes {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    upsert_pub_file(&mut app.zlibs_pub_files, name, text);
                } else {
                    app.zlibs_error = Some(format!("El archivo '{}' no es texto UTF-8 válido.", name));
                }
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = f.path {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        upsert_pub_file(&mut app.zlibs_pub_files, name, text);
                    }
                }
            }
        }
    }

    card_frame(p).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("📦 ZLibs — Registro de Librerías Zen");
            badge_pill(
                ui,
                "GitHub + Zen CLI",
                p.accent_secondary.gamma_multiply(0.18),
                p.accent_secondary,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("↻ Actualizar").clicked() {
                    do_search = true;
                    if has_tok {
                        fetch_mine = true;
                    }
                }
            });
        });
        ui.label(
            egui::RichText::new(
                "Explora paquetes de Zen, genera tu token para `zen login` y publica librerías directamente en repositorios públicos de GitHub.",
            )
            .small()
            .color(p.text_muted),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(app.zlibs_tab == 0, "🔍 Explorar librerías")
                .clicked()
            {
                app.zlibs_tab = 0;
            }
            if ui
                .selectable_label(app.zlibs_tab == 1, "🔑 Mis paquetes y CLI")
                .clicked()
            {
                app.zlibs_tab = 1;
                if has_tok && app.zlibs_account.is_none() {
                    fetch_mine = true;
                }
            }
            if ui
                .selectable_label(app.zlibs_tab == 2, "🚀 Publicar librería")
                .clicked()
            {
                app.zlibs_tab = 2;
            }
        });
    });

    if let Some(err) = &app.zlibs_error {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("⚠️ {err}")).small().color(p.danger));
    }
    if let Some(info) = &app.zlibs_info {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("✅ {info}")).small().color(p.success));
    }

    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .id_salt("zlibs_scroll")
        .show(ui, |ui| match app.zlibs_tab {
            0 => {
                // ===== Explorar =====
                card_frame(p).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut app.zlibs_search)
                                .desired_width(ui.available_width().min(320.0))
                                .hint_text("Buscar librería Zen por nombre, descripción o autor…"),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            do_search = true;
                        }
                        if primary_button(ui, p, "Buscar").clicked() {
                            do_search = true;
                        }
                        if !app.zlibs_search.is_empty() && ui.button("Limpiar").clicked() {
                            app.zlibs_search.clear();
                            do_search = true;
                        }
                    });
                });

                ui.add_space(8.0);

                if let Some(detail) = app.zlibs_active_detail.clone() {
                    card_frame(p).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("← Volver al listado").clicked() {
                                app.zlibs_active_detail = None;
                            }
                            ui.heading(format!("📦 {}", detail.package.name));
                            badge_pill(
                                ui,
                                &format!("v{}", detail.package.latest_version),
                                p.accent_secondary.gamma_multiply(0.18),
                                p.accent_secondary,
                            );
                            if ui.button("GitHub ↗").clicked() {
                                ctx.open_url(egui::OpenUrl::new_tab(detail.package.git_url.clone()));
                            }
                        });
                        ui.label(
                            egui::RichText::new(&detail.package.description).color(p.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Autor: @{} · {} descargas · Actualizado: {}",
                                detail.package.owner_username,
                                detail.package.downloads_count,
                                detail.package.updated_at
                            ))
                            .small()
                            .color(p.text_muted),
                        );
                        ui.add_space(8.0);
                        elevated_frame(p).show(ui, |ui| {
                            ui.label(egui::RichText::new("Línea para Make.mz:").small().color(p.text_muted));
                            ui.horizontal_wrapped(|ui| {
                                ui.monospace(&detail.package.dep_line);
                                if ui.button("📋 Copiar").clicked() {
                                    ctx.copy_text(detail.package.dep_line.clone());
                                    app.status_banner = Some("Línea dep copiada al portapapeles.".into());
                                }
                            });
                        });

                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "🏷️ Versiones ({})",
                                detail.package.versions.len()
                            ))
                            .strong(),
                        );
                        ui.add_space(4.0);
                        for v in &detail.package.versions {
                            elevated_frame(p).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    badge_pill(
                                        ui,
                                        &v.version,
                                        p.accent_primary.gamma_multiply(0.16),
                                        p.accent_primary,
                                    );
                                    ui.monospace(&v.dep_line);
                                    if ui.button("Copiar dep").clicked() {
                                        ctx.copy_text(v.dep_line.clone());
                                        app.status_banner = Some(format!("Copiado: {}", v.dep_line));
                                    }
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} archivos · {}",
                                            v.files_count, v.created_at
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                        }

                        if !detail.manifest.trim().is_empty() {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("⚙️ Manifiesto (Make.mz / Manifest.zm)").strong());
                            elevated_frame(p).show(ui, |ui| {
                                ui.monospace(&detail.manifest);
                            });
                        }

                        if !detail.readme.trim().is_empty() {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("📄 README.md").strong());
                            elevated_frame(p).show(ui, |ui| {
                                ui.label(egui::RichText::new(&detail.readme).monospace().small());
                            });
                        }
                    });
                } else if app.zlibs_packages.is_empty() {
                    card_frame(p).show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(
                                "No se encontraron librerías Zen. Publica la primera con `zen publicar` o desde la pestaña Publicar.",
                            )
                            .color(p.text_muted),
                        );
                    });
                } else {
                    for pkg in &app.zlibs_packages {
                        card_frame(p).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(&pkg.name)
                                        .strong()
                                        .size(16.0)
                                        .color(p.accent_primary),
                                );
                                badge_pill(
                                    ui,
                                    &format!("v{}", pkg.latest_version),
                                    p.accent_secondary.gamma_multiply(0.16),
                                    p.accent_secondary,
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "@{} · {} descargas",
                                                pkg.owner_username, pkg.downloads_count
                                            ))
                                            .small()
                                            .color(p.text_muted),
                                        );
                                    },
                                );
                            });
                            ui.label(&pkg.description);
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.monospace(&pkg.dep_line);
                                if ui.button("📋 Copiar dep").clicked() {
                                    ctx.copy_text(pkg.dep_line.clone());
                                    app.status_banner = Some(format!("Copiado: {}", pkg.dep_line));
                                }
                                if ui.button("GitHub ↗").clicked() {
                                    ctx.open_url(egui::OpenUrl::new_tab(pkg.git_url.clone()));
                                }
                                if primary_button(ui, p, "Ver detalles").clicked() {
                                    fetch_detail = Some(pkg.name.clone());
                                }
                            });
                        });
                        ui.add_space(8.0);
                    }
                }
            }
            1 => {
                // ===== Mis paquetes y Token CLI =====
                if !has_tok || is_guest {
                    card_frame(p).show(ui, |ui| {
                        ui.heading("🔐 Sesión requerida");
                        ui.label(
                            egui::RichText::new(
                                "Inicia sesión con tu cuenta de Aethravox para generar tokens CLI y gestionar tus librerías publicadas.",
                            )
                            .color(p.text_muted),
                        );
                    });
                    return;
                }

                card_frame(p).show(ui, |ui| {
                    ui.heading("🔑 Generador de Token para Zen CLI");
                    ui.label(
                        egui::RichText::new(
                            "El token se guarda hasheado con SHA-256 en el servidor y solo se muestra en texto plano al crearlo.",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.zlibs_token_label)
                                .desired_width(220.0)
                                .hint_text("Etiqueta (ej: Portátil Linux)"),
                        );
                        if primary_button(ui, p, "🔑 Generar Token CLI").clicked() {
                            create_token = true;
                        }
                    });

                    if let Some(created) = &app.zlibs_created_token {
                        ui.add_space(10.0);
                        egui::Frame::none()
                            .fill(p.accent_primary.gamma_multiply(0.12))
                            .stroke(egui::Stroke::new(1.0_f32, p.accent_primary))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Token creado (cópialo ahora — no volverá a mostrarse):",
                                    )
                                    .strong()
                                    .color(p.accent_primary),
                                );
                                ui.monospace(&created.cli_command);
                                ui.horizontal_wrapped(|ui| {
                                    if primary_button(ui, p, "📋 Copiar comando zen login").clicked() {
                                        ctx.copy_text(created.cli_command.clone());
                                        app.status_banner = Some("Comando zen login copiado.".into());
                                    }
                                    if ui.button("Copiar solo token").clicked() {
                                        ctx.copy_text(created.token.clone());
                                        app.status_banner = Some("Token CLI copiado.".into());
                                    }
                                });
                            });
                    }

                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Tokens activos:").strong().small());
                    if let Some(acc) = &app.zlibs_account {
                        if acc.tokens.is_empty() {
                            ui.label(
                                egui::RichText::new("No tienes tokens CLI activos todavía.")
                                    .small()
                                    .color(p.text_muted),
                            );
                        } else {
                            for t in &acc.tokens {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(&t.label).strong());
                                        ui.monospace(&t.token_prefix);
                                        let last = t
                                            .last_used_at
                                            .as_deref()
                                            .unwrap_or("sin uso");
                                        ui.label(
                                            egui::RichText::new(format!("· último uso: {last}"))
                                                .small()
                                                .color(p.text_muted),
                                        );
                                        if ui
                                            .button(egui::RichText::new("Revocar").color(p.danger))
                                            .clicked()
                                        {
                                            revoke_token_id = Some(t.id);
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("Cargando tokens…").small().color(p.text_muted));
                    }
                });

                ui.add_space(10.0);

                card_frame(p).show(ui, |ui| {
                    ui.heading("📚 Mis Paquetes Publicados");
                    if let Some(acc) = &app.zlibs_account {
                        if acc.my_packages.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "Aún no has publicado paquetes. Ejecuta `zen publicar` desde tu proyecto o usa la pestaña Publicar.",
                                )
                                .small()
                                .color(p.text_muted),
                            );
                        } else {
                            for pkg in &acc.my_packages {
                                elevated_frame(p).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(&pkg.name)
                                                .strong()
                                                .color(p.accent_primary),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} descargas",
                                                pkg.downloads_count
                                            ))
                                            .small()
                                            .color(p.text_muted),
                                        );
                                        if ui.button("GitHub ↗").clicked() {
                                            ctx.open_url(egui::OpenUrl::new_tab(pkg.git_url.clone()));
                                        }
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        for v in &pkg.versions {
                                            if ui
                                                .button(format!("v{}", v.version))
                                                .on_hover_text(format!("Copiar {}", v.dep_line))
                                                .clicked()
                                            {
                                                ctx.copy_text(v.dep_line.clone());
                                                app.status_banner =
                                                    Some(format!("Copiado: {}", v.dep_line));
                                            }
                                        }
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        ui.monospace(&pkg.dep_line);
                                        if ui.button("📋 Copiar Make.mz").clicked() {
                                            ctx.copy_text(pkg.dep_line.clone());
                                            app.status_banner =
                                                Some(format!("Copiado: {}", pkg.dep_line));
                                        }
                                    });
                                });
                                ui.add_space(6.0);
                            }
                        }
                    }
                });
            }
            _ => {
                // ===== Publicar librería =====
                if !has_tok || is_guest {
                    card_frame(p).show(ui, |ui| {
                        ui.heading("🔐 Sesión requerida");
                        ui.label(
                            egui::RichText::new(
                                "Inicia sesión para publicar librerías Zen en GitHub y ZLibs.",
                            )
                            .color(p.text_muted),
                        );
                    });
                    return;
                }

                card_frame(p).show(ui, |ui| {
                    ui.heading("🚀 Publicar librería Zen en GitHub");
                    ui.label(
                        egui::RichText::new(
                            "Al publicar, el servidor creará automáticamente el repositorio público en GitHub (si no existe) y etiquetará la versión.",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Nombre ([a-zA-Z0-9_-]):");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.zlibs_pub_name)
                                    .desired_width(220.0)
                                    .hint_text("mi_libreria"),
                            );
                        });
                        ui.vertical(|ui| {
                            ui.label("Versión:");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.zlibs_pub_version)
                                    .desired_width(120.0)
                                    .hint_text("1.0.0"),
                            );
                        });
                    });
                    ui.add_space(6.0);
                    ui.label("Descripción (opcional):");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.zlibs_pub_desc)
                            .desired_width(ui.available_width().min(480.0))
                            .hint_text("Descripción corta de la librería"),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "Arrastra archivos (.zen, Make.mz, Manifest.zm, README.md) sobre esta ventana o edita los archivos iniciales abajo:",
                        )
                        .small()
                        .color(p.text_muted),
                    );
                    if !app.zlibs_pub_files.is_empty() {
                        ui.add_space(4.0);
                        let mut remove_idx: Option<usize> = None;
                        for (idx, f) in app.zlibs_pub_files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.monospace(&f.path);
                                ui.label(
                                    egui::RichText::new(format!("({} bytes)", f.content.len()))
                                        .small()
                                        .color(p.text_muted),
                                );
                                if ui.small_button("✕").clicked() {
                                    remove_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            app.zlibs_pub_files.remove(idx);
                        }
                    }

                    ui.add_space(8.0);
                    ui.label("Main.zen (opcional si arrastraste archivos):");
                    ui.add(
                        egui::TextEdit::multiline(&mut app.zlibs_pub_main)
                            .desired_rows(5)
                            .desired_width(ui.available_width())
                            .code_editor(),
                    );
                    ui.add_space(6.0);
                    ui.label("Make.mz (opcional):");
                    ui.add(
                        egui::TextEdit::multiline(&mut app.zlibs_pub_manifest)
                            .desired_rows(3)
                            .desired_width(ui.available_width())
                            .code_editor(),
                    );
                    ui.add_space(6.0);
                    ui.label("README.md (opcional):");
                    ui.add(
                        egui::TextEdit::multiline(&mut app.zlibs_pub_readme)
                            .desired_rows(3)
                            .desired_width(ui.available_width())
                            .code_editor(),
                    );

                    ui.add_space(10.0);
                    if app.zlibs_busy {
                        ui.label(
                            egui::RichText::new("⏳ Creando repositorio/tag en GitHub y registrando en ZLibs…")
                                .color(p.accent_secondary),
                        );
                    } else if primary_button(ui, p, "🚀 Publicar librería").clicked() {
                        publish_now = true;
                    }
                });
            }
        });

    if do_search && !app.offline_mode {
        spawn_fetch_zlibs_packages(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            app.zlibs_search.clone(),
        );
    }
    if let Some(name) = fetch_detail {
        if !app.offline_mode {
            spawn_fetch_zlib_detail(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                name,
            );
        }
    }
    if fetch_mine && has_tok {
        spawn_fetch_zlibs_mine(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if create_token && has_tok {
        spawn_create_zlib_token(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
            app.zlibs_token_label.clone(),
        );
    }
    if let Some(tid) = revoke_token_id {
        if has_tok {
            spawn_revoke_zlib_token(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                tid,
            );
        }
    }
    if publish_now && has_tok {
        let mut files = app.zlibs_pub_files.clone();
        if !app.zlibs_pub_main.trim().is_empty()
            && !files.iter().any(|f| f.path.eq_ignore_ascii_case("Main.zen"))
        {
            files.push(ZLibFileInput {
                path: "Main.zen".into(),
                content: app.zlibs_pub_main.clone(),
            });
        }
        if !app.zlibs_pub_manifest.trim().is_empty()
            && !files.iter().any(|f| f.path.eq_ignore_ascii_case("Make.mz"))
        {
            files.push(ZLibFileInput {
                path: "Make.mz".into(),
                content: app.zlibs_pub_manifest.clone(),
            });
        }
        if !app.zlibs_pub_readme.trim().is_empty()
            && !files.iter().any(|f| f.path.eq_ignore_ascii_case("README.md"))
        {
            files.push(ZLibFileInput {
                path: "README.md".into(),
                content: app.zlibs_pub_readme.clone(),
            });
        }
        if files.is_empty() {
            app.zlibs_error = Some("Agrega al menos un archivo o contenido en Main.zen.".into());
        } else {
            app.zlibs_error = None;
            app.zlibs_info = None;
            app.zlibs_busy = true;
            let desc = if app.zlibs_pub_desc.trim().is_empty() {
                None
            } else {
                Some(app.zlibs_pub_desc.trim().to_string())
            };
            spawn_publish_zlib(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                ZLibPublishRequest {
                    name: app.zlibs_pub_name.trim().to_string(),
                    version: app.zlibs_pub_version.trim().to_string(),
                    description: desc,
                    files,
                },
            );
        }
    }
}

fn upsert_pub_file(list: &mut Vec<ZLibFileInput>, path: String, content: String) {
    if let Some(existing) = list.iter_mut().find(|f| f.path == path) {
        existing.content = content;
    } else {
        list.push(ZLibFileInput { path, content });
    }
}
