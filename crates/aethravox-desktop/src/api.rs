use aethravox_core::{
    ApiError, ArisChatDto, ArisMessageDto, AuthSuccess, CategoryRowDto, ChatConversationDto,
    ChatMessageDto, HubPayload, HubStatsDto, LoginRequest, RegisterRequest, SendMessageRequest,
    ThemeKind, ThemeSelectRequest, UserSummary, WebAppDto,
};

#[derive(Debug, Clone)]
pub enum ApiEvent {
    AuthOk(AuthSuccess),
    AuthErr(String),
    HubLoaded(HubPayload),
    HubErr(String),
    #[allow(dead_code)]
    ThemeSaved(ThemeKind),
    #[allow(dead_code)]
    StatusNote(String),
    ConversationsLoaded(Vec<ChatConversationDto>),
    MessagesLoaded {
        conv_id: i64,
        messages: Vec<ChatMessageDto>,
    },
    MessageSent(ChatMessageDto),
    ChatErr(String),
    ChatSendErr(String),
}

pub fn spawn_login(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: LoginRequest,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/auth/login", base_url.trim_end_matches('/'));
        match client.post(&url).json(&req).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<AuthSuccess>().await {
                        Ok(data) => {
                            let _ = tx.send(ApiEvent::AuthOk(data));
                        }
                        Err(e) => {
                            let _ = tx.send(ApiEvent::AuthErr(format!("Error parseando respuesta: {e}")));
                        }
                    }
                } else {
                    let msg = resp
                        .json::<ApiError>()
                        .await
                        .map(|e| e.error)
                        .unwrap_or_else(|_| "Credenciales inválidas.".into());
                    let _ = tx.send(ApiEvent::AuthErr(msg));
                }
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(format!(
                    "No se pudo conectar al servidor ({url}): {e}"
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_register(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: RegisterRequest,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/auth/register", base_url.trim_end_matches('/'));
        match client.post(&url).json(&req).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<AuthSuccess>().await {
                        Ok(data) => {
                            let _ = tx.send(ApiEvent::AuthOk(data));
                        }
                        Err(e) => {
                            let _ = tx.send(ApiEvent::AuthErr(format!("Error parseando respuesta: {e}")));
                        }
                    }
                } else {
                    let msg = resp
                        .json::<ApiError>()
                        .await
                        .map(|e| e.error)
                        .unwrap_or_else(|_| "No se pudo registrar.".into());
                    let _ = tx.send(ApiEvent::AuthErr(msg));
                }
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(format!(
                    "No se pudo conectar al servidor ({url}): {e}"
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_hub(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/hub", base_url.trim_end_matches('/'));
        let mut req = client.get(&url);
        if let Some(tok) = token {
            if !tok.is_empty() {
                req = req.bearer_auth(tok);
            }
        }
        match req.send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<HubPayload>().await {
                Ok(hub) => {
                    let _ = tx.send(ApiEvent::HubLoaded(hub));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::HubErr(format!("Error parseando hub: {e}")));
                }
            },
            Ok(resp) => {
                let _ = tx.send(ApiEvent::HubErr(format!("HTTP {}", resp.status())));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::HubErr(format!("Servidor no disponible: {e}")));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_save_theme(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    theme: ThemeKind,
) {
    rt.spawn(async move {
        if let Some(tok) = token {
            if !tok.is_empty() {
                let client = reqwest::Client::new();
                let url = format!("{}/api/v1/theme", base_url.trim_end_matches('/'));
                let _ = client
                    .post(&url)
                    .bearer_auth(tok)
                    .json(&ThemeSelectRequest {
                        theme: theme.as_str().to_string(),
                    })
                    .send()
                    .await;
            }
        }
        let _ = tx.send(ApiEvent::ThemeSaved(theme));
        ctx.request_repaint();
    });
}

// ===================== Chat =====================

async fn read_error(resp: reqwest::Response) -> String {
    let status = resp.status();
    resp.json::<ApiError>()
        .await
        .map(|e| e.error)
        .unwrap_or_else(|_| format!("HTTP {status}"))
}

async fn get_json<T: serde::de::DeserializeOwned>(
    base_url: &str,
    path: &str,
    token: &Option<String>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut req = client.get(&url);
    if let Some(tok) = token {
        if !tok.is_empty() {
            req = req.bearer_auth(tok);
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<T>()
            .await
            .map_err(|e| format!("Error parseando respuesta: {e}")),
        Ok(resp) => Err(read_error(resp).await),
        Err(e) => Err(format!("Servidor no disponible: {e}")),
    }
}

pub fn spawn_fetch_conversations(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<ChatConversationDto>>(&base_url, "/api/v1/chat/conversations", &token)
            .await
        {
            Ok(list) => {
                let _ = tx.send(ApiEvent::ConversationsLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_messages(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/messages");
        match get_json::<Vec<ChatMessageDto>>(&base_url, &path, &token).await {
            Ok(messages) => {
                let _ = tx.send(ApiEvent::MessagesLoaded { conv_id, messages });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_send_message(
    rt: &tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    body: String,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/v1/chat/conversations/{}/messages",
            base_url.trim_end_matches('/'),
            conv_id
        );
        let mut req = client.post(&url).json(&SendMessageRequest { body });
        if let Some(tok) = token {
            if !tok.is_empty() {
                req = req.bearer_auth(tok);
            }
        }
        match req.send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<ChatMessageDto>().await {
                Ok(msg) => {
                    let _ = tx.send(ApiEvent::MessageSent(msg));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::ChatSendErr(format!(
                        "Error parseando respuesta: {e}"
                    )));
                }
            },
            Ok(resp) => {
                let _ = tx.send(ApiEvent::ChatSendErr(read_error(resp).await));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatSendErr(format!(
                    "Servidor no disponible: {e}"
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_mark_read(
    rt: &tokio::runtime::Handle,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/v1/chat/conversations/{}/read",
            base_url.trim_end_matches('/'),
            conv_id
        );
        let mut req = client.post(&url);
        if let Some(tok) = token {
            if !tok.is_empty() {
                req = req.bearer_auth(tok);
            }
        }
        let _ = req.send().await;
    });
}

/// Datos locales por defecto para probar el cliente desktop de inmediato sin servidor.
pub fn offline_demo_hub(username: &str, is_guest: bool, theme: ThemeKind) -> HubPayload {
    let user = UserSummary {
        id: if is_guest { 0 } else { 1 },
        username: username.to_string(),
        display: username.to_string(),
        role_key: if is_guest { "invitado" } else { "owner" }.to_string(),
        role_display: if is_guest { "Invitado" } else { "Owner" }.to_string(),
        role_color: if is_guest { "#6b7280" } else { "#ef4444" }.to_string(),
        role_text_color: "#ffffff".to_string(),
        is_guest,
        can_access_admin: !is_guest,
        bio: "Modo vista previa local de Aethravox Desktop (egui).".to_string(),
        theme,
    };

    let app = |id: i64, slug: &str, name: &str, desc: &str, icon: &str, tag: &str| WebAppDto {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        icon: icon.to_string(),
        url: format!("/{slug}"),
        tag: tag.to_string(),
        tag_label: if tag.is_empty() {
            "Activo".to_string()
        } else {
            tag.to_string()
        },
        tag_color: match tag {
            "Nuevo" => "#00d4ff",
            "Beta" => "#ff3e3e",
            "Próximamente" => "#e0a308",
            _ => "#00ff88",
        }
        .to_string(),
        clickable: tag != "Próximamente",
        lock_message: "Esta sección estará disponible pronto.".to_string(),
    };

    let categories = vec![
        CategoryRowDto {
            name: "Inteligencia Artificial".into(),
            slug: "ia".into(),
            items: vec![app(
                1,
                "aris",
                "Aris",
                "Tu asistente de inteligencia artificial. Conversa, pregunta, crea.",
                "✨",
                "Nuevo",
            )],
        },
        CategoryRowDto {
            name: "Comunicación".into(),
            slug: "comunicacion".into(),
            items: vec![
                app(2, "chat", "Mensajería", "Chat privado y grupal en tiempo real.", "💬", ""),
                app(3, "friends", "Amigos", "Lista de amigos y solicitudes pendientes.", "👥", ""),
                app(4, "communities", "Comunidades", "Espacios con múltiples grupos organizados.", "🌐", ""),
            ],
        },
        CategoryRowDto {
            name: "Entretenimiento".into(),
            slug: "entretenimiento".into(),
            items: vec![app(
                5,
                "neon-canvas",
                "Neon Canvas",
                "Juego de dibujo neón 2D nativo en egui Painter.",
                "🎨",
                "Beta",
            )],
        },
        CategoryRowDto {
            name: "Creatividad".into(),
            slug: "creatividad".into(),
            items: vec![
                app(6, "docs", "Documentos", "Galería y visor nativo de documentos.", "📄", ""),
                app(7, "aextube", "aexTube", "Plataforma de vídeo del estudio.", "📺", ""),
                app(8, "playground", "Playground", "Laboratorio interactivo de código.", "🧪", ""),
            ],
        },
        CategoryRowDto {
            name: "Herramientas".into(),
            slug: "herramientas".into(),
            items: vec![
                app(9, "aexos", "aexOS", "Entorno de escritorio interactivo.", "🖥", ""),
                app(10, "transmitir", "Transmitir", "Comparte tu pantalla en red local.", "📡", ""),
                app(11, "descargas", "Descargas", "Clientes nativos y releases.", "⬇", ""),
            ],
        },
        CategoryRowDto {
            name: "Asistencia".into(),
            slug: "asistencia".into(),
            items: vec![app(
                12,
                "support",
                "Asistencia",
                "Envía sugerencias, reportes o consultas al equipo.",
                "🛟",
                "",
            )],
        },
    ];

    HubPayload {
        user,
        categories,
        stats: HubStatsDto {
            online_count: 1,
            doc_count: 4,
            chat_unread: 2,
            community_unread: 0,
            owner_count: 1,
            admin_count: 0,
            staff_count: 0,
            user_count: 1,
        },
    }
}

pub fn demo_chat_conversations() -> Vec<ChatConversationDto> {
    vec![
        ChatConversationDto {
            id: 1,
            title: "Equipo Aethravox".into(),
            is_group: true,
            last_message: "Migrando la UI a egui nativo 🚀".into(),
            unread_count: 2,
        },
        ChatConversationDto {
            id: 2,
            title: "Aris · Notas rápidas".into(),
            is_group: false,
            last_message: "Listo el sistema de temas.".into(),
            unread_count: 0,
        },
    ]
}

pub fn demo_chat_messages() -> Vec<ChatMessageDto> {
    vec![
        ChatMessageDto {
            id: 1,
            conversation_id: 1,
            sender: "Aethravox".into(),
            content: "Bienvenido al cliente nativo de Aethravox Studios en Rust + egui.".into(),
            created_at: "13:00".into(),
            is_mine: false,
        },
        ChatMessageDto {
            id: 2,
            conversation_id: 1,
            sender: "Tú".into(),
            content: "Adiós Askama y CSS, ahora todo corre directo en GPU.".into(),
            created_at: "13:02".into(),
            is_mine: true,
        },
    ]
}

pub fn demo_aris_chats() -> (Vec<ArisChatDto>, Vec<ArisMessageDto>) {
    (
        vec![ArisChatDto {
            id: 1,
            title: "Arquitectura Desktop".into(),
            pinned: true,
            updated_at: "Ahora".into(),
        }],
        vec![ArisMessageDto {
            id: 1,
            role: "assistant".into(),
            content: "Hola, soy Aris. Estoy lista en el nuevo cliente nativo de Aethravox Studios."
                .into(),
            model: Some("aris-core".into()),
            created_at: "Ahora".into(),
        }],
    )
}
