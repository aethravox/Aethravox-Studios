use aethravox_core::{
    ArisChatDto, ArisMessageDto, ArisMode, ChatConversationDto, ChatMessageDto, ColorMode,
    HubPayload, ThemeKind, WebAppDto,
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::api::{
    demo_aris_chats, demo_chat_conversations, demo_chat_messages, offline_demo_hub,
    spawn_fetch_conversations, spawn_fetch_messages, spawn_mark_read, spawn_send_message,
    ApiEvent,
};
use crate::theme::{apply_theme, Palette};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Login,
    Hub,
    Aris,
    Chat,
    NeonCanvas,
    Settings,
    Placeholder(&'static str),
}

#[derive(Debug, Clone)]
pub struct NeonStroke {
    pub points: Vec<egui::Pos2>,
    pub color: egui::Color32,
    pub width: f32,
}

pub struct AethravoxApp {
    pub rt: tokio::runtime::Handle,
    pub tx: mpsc::UnboundedSender<ApiEvent>,
    pub rx: mpsc::UnboundedReceiver<ApiEvent>,

    // Conexión y sesión
    pub server_url: String,
    pub token: Option<String>,
    pub route: Route,
    pub offline_mode: bool,

    // Tema visual
    pub theme: ThemeKind,
    pub color_mode: ColorMode,
    pub sidebar_collapsed: bool,

    // Login / Register state
    pub auth_tab_register: bool,
    pub login_username: String,
    pub login_password: String,
    pub register_email: String,
    pub auth_error: Option<String>,
    pub auth_busy: bool,

    // Hub state
    pub hub: Option<HubPayload>,
    pub hub_search: String,
    pub hub_cat_filter: String,
    pub locked_modal: Option<(String, String)>,

    // Chat state
    pub conversations: Vec<ChatConversationDto>,
    pub active_conv_id: i64,
    pub chat_messages: Vec<ChatMessageDto>,
    pub chat_draft: String,
    pub chat_error: Option<String>,
    pub chat_sending: bool,
    pub chat_pending: Option<String>,
    pub chat_last_poll: Option<Instant>,

    // Aris state
    pub aris_chats: Vec<ArisChatDto>,
    pub active_aris_chat_id: i64,
    pub aris_messages: Vec<ArisMessageDto>,
    pub aris_mode: ArisMode,
    pub aris_draft: String,

    // Neon Canvas state
    pub neon_strokes: Vec<NeonStroke>,
    pub neon_active_stroke: Option<NeonStroke>,
    pub neon_color: egui::Color32,
    pub neon_width: f32,

    // Status toast
    pub status_banner: Option<String>,
}

impl AethravoxApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        tx: mpsc::UnboundedSender<ApiEvent>,
        rx: mpsc::UnboundedReceiver<ApiEvent>,
        rt: tokio::runtime::Handle,
    ) -> Self {
        let theme = ThemeKind::Aethravox;
        let color_mode = ColorMode::Dark;
        apply_theme(&cc.egui_ctx, theme, color_mode);

        let (aris_chats, aris_messages) = demo_aris_chats();

        Self {
            rt,
            tx,
            rx,
            server_url: "https://aethravoxstudios.duckdns.org".to_string(),
            token: None,
            route: Route::Login,
            offline_mode: false,
            theme,
            color_mode,
            sidebar_collapsed: false,
            auth_tab_register: false,
            login_username: String::new(),
            login_password: String::new(),
            register_email: String::new(),
            auth_error: None,
            auth_busy: false,
            hub: None,
            hub_search: String::new(),
            hub_cat_filter: "all".to_string(),
            locked_modal: None,
            conversations: demo_chat_conversations(),
            active_conv_id: 1,
            chat_messages: demo_chat_messages(),
            chat_draft: String::new(),
            chat_error: None,
            chat_sending: false,
            chat_pending: None,
            chat_last_poll: None,
            aris_chats,
            active_aris_chat_id: 1,
            aris_messages,
            aris_mode: ArisMode::Instantaneo,
            aris_draft: String::new(),
            neon_strokes: Vec::new(),
            neon_active_stroke: None,
            neon_color: egui::Color32::from_rgb(0, 212, 255),
            neon_width: 4.0,
            status_banner: None,
        }
    }

    pub fn set_theme(&mut self, ctx: &egui::Context, theme: ThemeKind) {
        self.theme = theme;
        apply_theme(ctx, self.theme, self.color_mode);
        if !self.offline_mode {
            crate::api::spawn_save_theme(
                &self.rt,
                self.tx.clone(),
                ctx.clone(),
                self.server_url.clone(),
                self.token.clone(),
                theme,
            );
        }
    }

    pub fn set_color_mode(&mut self, ctx: &egui::Context, mode: ColorMode) {
        self.color_mode = mode;
        apply_theme(ctx, self.theme, self.color_mode);
    }

    pub fn enter_offline_demo(&mut self, username: &str, is_guest: bool) {
        self.offline_mode = true;
        self.token = None;
        self.auth_error = None;
        self.auth_busy = false;
        self.hub = Some(offline_demo_hub(username, is_guest, self.theme));
        self.conversations = demo_chat_conversations();
        self.chat_messages = demo_chat_messages();
        self.active_conv_id = 1;
        self.chat_error = None;
        self.route = Route::Hub;
    }

    pub fn open_app(&mut self, app: &WebAppDto) {
        if !app.clickable {
            self.locked_modal = Some((app.name.clone(), app.lock_message.clone()));
            return;
        }
        match app.slug.as_str() {
            "aris" => self.route = Route::Aris,
            "chat" => self.route = Route::Chat,
            "neon-canvas" => self.route = Route::NeonCanvas,
            "dashboard" | "perfil" => self.route = Route::Settings,
            "friends" => self.route = Route::Placeholder("Amigos"),
            "communities" => self.route = Route::Placeholder("Comunidades"),
            "docs" => self.route = Route::Placeholder("Documentos"),
            "aexos" => self.route = Route::Placeholder("aexOS Desktop"),
            "aextube" => self.route = Route::Placeholder("aexTube"),
            "playground" => self.route = Route::Placeholder("Playground"),
            "transmitir" => self.route = Route::Placeholder("Transmitir"),
            "descargas" => self.route = Route::Placeholder("Descargas"),
            "support" => self.route = Route::Placeholder("Asistencia"),
            _ => self.route = Route::Placeholder("Sección en migración a egui"),
        }
    }

    pub fn logout(&mut self) {
        self.token = None;
        self.hub = None;
        self.offline_mode = false;
        self.login_password.clear();
        self.reset_chat_state();
        self.route = Route::Login;
    }

    fn reset_chat_state(&mut self) {
        self.conversations.clear();
        self.chat_messages.clear();
        self.active_conv_id = 0;
        self.chat_draft.clear();
        self.chat_error = None;
        self.chat_sending = false;
        self.chat_pending = None;
        self.chat_last_poll = None;
    }

    /// Abre una conversación: carga sus mensajes y la marca como leída (modo online).
    pub fn chat_open_conversation(&mut self, ctx: &egui::Context, conv_id: i64) {
        self.active_conv_id = conv_id;
        self.chat_error = None;
        if self.offline_mode || self.token.is_none() || conv_id == 0 {
            return;
        }
        spawn_fetch_messages(
            &self.rt,
            self.tx.clone(),
            ctx.clone(),
            self.server_url.clone(),
            self.token.clone(),
            conv_id,
        );
        spawn_mark_read(
            &self.rt,
            self.server_url.clone(),
            self.token.clone(),
            conv_id,
        );
    }

    /// Envía el borrador actual. En modo local lo añade a la lista de demostración.
    pub fn chat_send_draft(&mut self, ctx: &egui::Context) {
        let txt = self.chat_draft.trim().to_string();
        if txt.is_empty() {
            return;
        }

        if self.offline_mode || self.token.is_none() {
            let next_id = self.chat_messages.len() as i64 + 1;
            let sender = self
                .hub
                .as_ref()
                .map(|h| h.user.display.clone())
                .unwrap_or_else(|| "Tú".into());
            self.chat_messages.push(ChatMessageDto {
                id: next_id,
                conversation_id: self.active_conv_id,
                sender,
                content: txt,
                created_at: "Ahora".into(),
                is_mine: true,
            });
            self.chat_draft.clear();
            return;
        }

        if self.active_conv_id == 0 || self.chat_sending {
            return;
        }
        self.chat_sending = true;
        self.chat_error = None;
        self.chat_pending = Some(txt.clone());
        self.chat_draft.clear();
        spawn_send_message(
            &self.rt,
            self.tx.clone(),
            ctx.clone(),
            self.server_url.clone(),
            self.token.clone(),
            self.active_conv_id,
            txt,
        );
    }

    /// Sondeo periódico mientras la pantalla de chat está abierta (modo online).
    fn chat_poll(&mut self, ctx: &egui::Context) {
        if self.route != Route::Chat || self.offline_mode || self.token.is_none() {
            self.chat_last_poll = None;
            return;
        }

        let due = self
            .chat_last_poll
            .map(|t| t.elapsed() >= Duration::from_secs(3))
            .unwrap_or(true);

        if due {
            self.chat_last_poll = Some(Instant::now());
            spawn_fetch_conversations(
                &self.rt,
                self.tx.clone(),
                ctx.clone(),
                self.server_url.clone(),
                self.token.clone(),
            );
            if self.active_conv_id != 0 {
                spawn_fetch_messages(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                    self.active_conv_id,
                );
                let has_unread = self
                    .conversations
                    .iter()
                    .any(|c| c.id == self.active_conv_id && c.unread_count > 0);
                if has_unread {
                    spawn_mark_read(
                        &self.rt,
                        self.server_url.clone(),
                        self.token.clone(),
                        self.active_conv_id,
                    );
                }
            }
        }

        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                ApiEvent::AuthOk(auth) => {
                    self.auth_busy = false;
                    self.auth_error = None;
                    self.offline_mode = false;
                    self.token = Some(auth.token.clone());
                    self.reset_chat_state();
                    self.theme = auth.user.theme;
                    apply_theme(ctx, self.theme, self.color_mode);
                    self.route = Route::Hub;
                    crate::api::spawn_fetch_hub(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::AuthErr(msg) => {
                    self.auth_busy = false;
                    self.auth_error = Some(msg);
                }
                ApiEvent::HubLoaded(hub) => {
                    self.theme = hub.user.theme;
                    apply_theme(ctx, self.theme, self.color_mode);
                    self.hub = Some(hub);
                }
                ApiEvent::HubErr(msg) => {
                    self.status_banner = Some(msg);
                }
                ApiEvent::ThemeSaved(_) => {}
                ApiEvent::StatusNote(note) => {
                    self.status_banner = Some(note);
                }
                ApiEvent::ConversationsLoaded(list) => {
                    self.chat_error = None;
                    self.conversations = list;
                    let active_exists = self
                        .conversations
                        .iter()
                        .any(|c| c.id == self.active_conv_id);
                    if !active_exists {
                        if let Some(first) = self.conversations.first().map(|c| c.id) {
                            self.chat_open_conversation(ctx, first);
                        } else {
                            self.active_conv_id = 0;
                        }
                    }
                }
                ApiEvent::MessagesLoaded { conv_id, messages } => {
                    self.chat_messages.retain(|m| m.conversation_id != conv_id);
                    self.chat_messages.extend(messages);
                }
                ApiEvent::MessageSent(msg) => {
                    self.chat_sending = false;
                    self.chat_pending = None;
                    if !self.chat_messages.iter().any(|m| m.id == msg.id) {
                        self.chat_messages.push(msg);
                    }
                    // Refrescar enseguida la lista y los mensajes.
                    self.chat_last_poll = None;
                }
                ApiEvent::ChatErr(msg) => {
                    self.chat_error = Some(msg);
                }
                ApiEvent::ChatSendErr(msg) => {
                    self.chat_sending = false;
                    self.chat_error = Some(msg);
                    if let Some(txt) = self.chat_pending.take() {
                        if self.chat_draft.is_empty() {
                            self.chat_draft = txt;
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for AethravoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);
        self.chat_poll(ctx);
        let p: Palette = Palette::from(self.theme, self.color_mode);

        if self.route == Route::Login {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(p.bg_base))
                .show(ctx, |ui| {
                    ui::login::show(self, ui, ctx, &p);
                });
            return;
        }

        // Top header bar
        egui::TopBottomPanel::top("top_header")
            .exact_height(54.0)
            .frame(
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui::sidebar::show_topbar(self, ui, ctx, &p);
            });

        // Left navigation sidebar
        let side_w = if self.sidebar_collapsed { 64.0 } else { 228.0 };
        egui::SidePanel::left("left_sidebar")
            .resizable(false)
            .exact_width(side_w)
            .frame(
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui::sidebar::show_sidebar(self, ui, ctx, &p);
            });

        // Main workspace panel
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_base)
                    .inner_margin(egui::Margin::same(22.0)),
            )
            .show(ctx, |ui| match self.route {
                Route::Login => {}
                Route::Hub => ui::hub::show(self, ui, &p),
                Route::Aris => ui::aris::show(self, ui, &p),
                Route::Chat => ui::chat::show(self, ui, &p),
                Route::NeonCanvas => ui::neon_canvas::show(self, ui, &p),
                Route::Settings => ui::settings::show(self, ui, ctx, &p),
                Route::Placeholder(title) => {
                    crate::theme::card_frame(&p).show(ui, |ui| {
                        ui.heading(title);
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Módulo preparado para migración nativa en egui dentro del workspace.",
                            )
                            .color(p.text_muted),
                        );
                        ui.add_space(12.0);
                        if ui.button("← Volver al Hub").clicked() {
                            self.route = Route::Hub;
                        }
                    });
                }
            });

        // Locked section modal
        if let Some((title, msg)) = self.locked_modal.clone() {
            let mut open = true;
            egui::Window::new(&title)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(&msg);
                    ui.add_space(12.0);
                    if crate::theme::primary_button(ui, &p, "Entendido").clicked() {
                        self.locked_modal = None;
                    }
                });
            if !open {
                self.locked_modal = None;
            }
        }
    }
}
