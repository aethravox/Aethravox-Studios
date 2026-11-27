use aethravox_core::CreateTicketRequest;

use crate::api::{
    spawn_create_ticket, spawn_fetch_tickets, spawn_reply_ticket, spawn_ticket_action,
    spawn_ticket_detail,
};
use crate::app::AethravoxApp;
use crate::theme::{badge_pill, card_frame, elevated_frame, primary_button, Palette};

pub fn show(app: &mut AethravoxApp, ui: &mut egui::Ui, p: &Palette) {
    let ctx = ui.ctx().clone();
    let mut refresh = false;
    let mut create_ticket = false;
    let mut select_ticket: Option<i64> = None;
    let mut send_reply: Option<i64> = None;
    let mut ticket_act: Option<(i64, &'static str)> = None;

    let is_guest = app.hub.as_ref().map(|h| h.user.is_guest).unwrap_or(true);

    ui.horizontal(|ui| {
        ui.heading("🛟 Asistencia y Tickets");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻ Actualizar").clicked() {
                refresh = true;
            }
        });
    });

    if let Some(err) = &app.support_error {
        ui.label(egui::RichText::new(err).small().color(p.danger));
    }
    ui.add_space(8.0);

    let mobile = app.is_mobile_layout(&ctx);

    if mobile {
        if !app.mobile_detail_open {
            show_ticket_list(
                app,
                ui,
                p,
                ui.available_width(),
                is_guest,
                &mut create_ticket,
                &mut select_ticket,
            );
        } else {
            show_ticket_detail(app, ui, p, true, &mut send_reply, &mut ticket_act);
        }
    } else {
        ui.horizontal(|ui| {
            show_ticket_list(
                app,
                ui,
                p,
                310.0,
                is_guest,
                &mut create_ticket,
                &mut select_ticket,
            );
            ui.add_space(10.0);
            show_ticket_detail(app, ui, p, false, &mut send_reply, &mut ticket_act);
        });
    }

    if refresh && !app.offline_mode {
        spawn_fetch_tickets(
            &app.rt,
            app.tx.clone(),
            ctx.clone(),
            app.server_url.clone(),
            app.token.clone(),
        );
    }
    if create_ticket {
        let req = CreateTicketRequest {
            ticket_type: app.ticket_type.clone(),
            subject: app.ticket_subject.trim().to_string(),
            body: app.ticket_body.trim().to_string(),
        };
        app.ticket_subject.clear();
        app.ticket_body.clear();
        if !app.offline_mode {
            spawn_create_ticket(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                req,
            );
        }
    }
    if let Some(id) = select_ticket {
        app.mobile_detail_open = true;
        if !app.offline_mode && app.token.is_some() {
            spawn_ticket_detail(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                id,
            );
        }
    }
    if let Some(id) = send_reply {
        let body = app.ticket_reply_draft.trim().to_string();
        app.ticket_reply_draft.clear();
        if !app.offline_mode && app.token.is_some() {
            spawn_reply_ticket(
                &app.rt,
                app.tx.clone(),
                ctx.clone(),
                app.server_url.clone(),
                app.token.clone(),
                id,
                body,
            );
        }
    }
    if let Some((id, act)) = ticket_act {
        if !app.offline_mode && app.token.is_some() {
            spawn_ticket_action(
                &app.rt,
                app.tx.clone(),
                ctx,
                app.server_url.clone(),
                app.token.clone(),
                id,
                act,
            );
        }
    }
}

fn show_ticket_list(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    width: f32,
    is_guest: bool,
    create_ticket: &mut bool,
    select_ticket: &mut Option<i64>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(width);
        ui.set_min_height(ui.available_height());

        ui.collapsing("+ Nuevo ticket / reporte", |ui| {
            ui.horizontal(|ui| {
                ui.label("Tipo:");
                let opts: &[(&str, &str)] = if is_guest {
                    &[("bug", "Bug"), ("sugerencia", "Sugerencia")]
                } else {
                    &[
                        ("bug", "Bug"),
                        ("sugerencia", "Sugerencia"),
                        ("soporte", "Soporte"),
                        ("otro", "Otro"),
                    ]
                };
                egui::ComboBox::from_id_salt("ticket_type_cb")
                    .selected_text(&app.ticket_type)
                    .show_ui(ui, |ui| {
                        for (val, label) in opts {
                            ui.selectable_value(
                                &mut app.ticket_type,
                                (*val).to_string(),
                                *label,
                            );
                        }
                    });
            });
            ui.add(
                egui::TextEdit::singleline(&mut app.ticket_subject)
                    .hint_text("Asunto breve…"),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.ticket_body)
                    .desired_rows(3)
                    .hint_text("Describe tu problema o sugerencia…"),
            );
            if primary_button(ui, p, "Enviar ticket").clicked()
                && !app.ticket_subject.trim().is_empty()
                && !app.ticket_body.trim().is_empty()
            {
                *create_ticket = true;
            }
        });

        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("ticket_list")
            .show(ui, |ui| {
                if app.tickets.is_empty() {
                    ui.label(
                        egui::RichText::new(if is_guest {
                            "Inicia sesión para consultar tu historial de tickets."
                        } else {
                            "No hay tickets registrados."
                        })
                        .small()
                        .color(p.text_muted),
                    );
                }
                let active_id = app.active_ticket.as_ref().map(|t| t.ticket.id);
                for t in &app.tickets {
                    let active = active_id == Some(t.id);
                    let resp = elevated_frame(p)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if active { p.accent_primary } else { p.border_subtle },
                        ))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("#{} {}", t.id, t.subject))
                                        .strong(),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                badge_pill(
                                    ui,
                                    &t.status,
                                    p.accent_secondary.gamma_multiply(0.16),
                                    p.accent_secondary,
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {}",
                                        t.ticket_type, t.created_at
                                    ))
                                    .small()
                                    .color(p.text_muted),
                                );
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        *select_ticket = Some(t.id);
                    }
                    ui.add_space(6.0);
                }
            });
    });
}

fn show_ticket_detail(
    app: &mut AethravoxApp,
    ui: &mut egui::Ui,
    p: &Palette,
    show_back: bool,
    send_reply: &mut Option<i64>,
    ticket_act: &mut Option<(i64, &'static str)>,
) {
    card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());

        if show_back {
            if ui.button("← Tickets").clicked() {
                app.mobile_detail_open = false;
            }
            ui.add_space(4.0);
        }

        if let Some(detail) = app.active_ticket.clone() {
            let t = &detail.ticket;
            ui.horizontal_wrapped(|ui| {
                ui.heading(format!("#{} — {}", t.id, t.subject));
                badge_pill(
                    ui,
                    &t.status,
                    p.accent_primary.gamma_multiply(0.18),
                    p.accent_primary,
                );
            });
            ui.label(
                egui::RichText::new(format!(
                    "Autor: {} · Tipo: {} · {}",
                    t.owner_name, t.ticket_type, t.created_at
                ))
                .small()
                .color(p.text_muted),
            );
            if !detail.reclamado_por_nombre.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "Atendido por: {}",
                        detail.reclamado_por_nombre
                    ))
                    .small()
                    .color(p.accent_secondary),
                );
            }

            if detail.is_admin {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Reclamar").clicked() {
                        *ticket_act = Some((t.id, "claim"));
                    }
                    if ui.button("Resolver").clicked() {
                        *ticket_act = Some((t.id, "resolve"));
                    }
                    if ui.button("Rechazar").clicked() {
                        *ticket_act = Some((t.id, "reject"));
                    }
                    if ui.button("Cerrar").clicked() {
                        *ticket_act = Some((t.id, "close"));
                    }
                });
            }

            ui.separator();

            let msg_h = (ui.available_height() - 70.0).max(140.0);
            egui::ScrollArea::vertical()
                .id_salt("ticket_msgs")
                .max_height(msg_h)
                .show(ui, |ui| {
                    elevated_frame(p).show(ui, |ui| {
                        ui.label(egui::RichText::new("Descripción inicial").strong().small());
                        ui.label(&t.body);
                    });
                    ui.add_space(6.0);

                    for m in &detail.messages {
                        elevated_frame(p)
                            .fill(if m.is_admin {
                                p.accent_primary.gamma_multiply(0.12)
                            } else {
                                p.bg_elevated
                            })
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(egui::RichText::new(&m.sender_name).strong().small());
                                    if m.is_admin {
                                        badge_pill(
                                            ui,
                                            "Staff",
                                            p.accent_primary.gamma_multiply(0.22),
                                            p.accent_primary,
                                        );
                                    }
                                    ui.label(
                                        egui::RichText::new(&m.created_at)
                                            .small()
                                            .color(p.text_muted),
                                    );
                                });
                                ui.label(&m.body);
                            });
                        ui.add_space(5.0);
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.ticket_reply_draft)
                        .desired_width((ui.available_width() - 110.0).max(120.0))
                        .hint_text("Responder en el ticket…"),
                );
                if primary_button(ui, p, "Responder").clicked()
                    && !app.ticket_reply_draft.trim().is_empty()
                {
                    *send_reply = Some(t.id);
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Selecciona un ticket o crea uno nuevo.")
                        .color(p.text_muted),
                );
            });
        }
    });
}
