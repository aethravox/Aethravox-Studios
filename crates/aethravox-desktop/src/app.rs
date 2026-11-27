use aethravox_core::{
    AdminApisPayload, AdminDownloadRowDto, AdminIpBansPayload, AdminOverviewDto, AdminRolesPayload,
    AdminTitlesPayload, AexosProfileDto,
    ArisChatDto, ArisCreditsDto, ArisMessageDto, ArisMode, ChatConversationDto, ChatGroupManageDto,
    ChatMessageDto, ChatMessageInfoDto, ColorMode, CommunityDetailDto, CommunityDto, DeviceProfile,
    DocumentDto, DownloadPlatformDto, EmailsPayload, FriendsPayload, HubPayload, LinkCodeDto,
    ProfilePayload, PublicProfileDto, SystemConfigDto, ThemeKind, TicketDetailDto, TicketDto,
    UserIpHistoryDto, UserSearchDto, VideoDto, WebAppDto, ZLibAccountPayload, ZLibCreateTokenResponse, ZLibFileInput, ZLibPackageDetailDto, ZLibPackageDto,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::api::{
    demo_aexos_profiles, demo_aris_chats, demo_chat_conversations, demo_chat_messages,
    demo_communities, demo_docs, demo_downloads, demo_friends, demo_tickets, demo_videos,
    offline_demo_hub, spawn_admin_apis, spawn_admin_config_get, spawn_admin_docs_list,
    spawn_admin_downloads_list, spawn_admin_ip_bans, spawn_admin_overview, spawn_admin_roles,
    spawn_admin_titles, spawn_aexos_profiles, spawn_aris_chats,
    spawn_aris_credits, spawn_aris_messages, spawn_community_detail, spawn_community_inviteable,
    spawn_fetch_communities, spawn_fetch_conversations, spawn_fetch_docs, spawn_fetch_downloads,
    spawn_fetch_emails, spawn_fetch_friends, spawn_fetch_hub, spawn_fetch_messages,
    demo_zlibs, spawn_fetch_profile, spawn_fetch_tickets, spawn_fetch_videos,
    spawn_fetch_zlibs_mine, spawn_fetch_zlibs_packages, spawn_logout, spawn_mark_read,
    spawn_save_theme, spawn_send_friend_request, spawn_send_message, spawn_start_dm,
    spawn_ticket_detail, ApiEvent, AsyncRt,
};
use crate::theme::{
    apply_theme_for_device, badge_pill, parse_hex_color, primary_button, Palette,
};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Login,
    Hub,
    Aris,
    Chat,
    Friends,
    Communities,
    Docs,
    NeonCanvas,
    AexTube,
    Playground,
    Aexos,
    Downloads,
    ZLibs,
    Support,
    Transmit,
    Settings,
    Admin,
}

#[derive(Debug, Clone)]
pub struct NeonStroke {
    pub points: Vec<egui::Pos2>,
    pub color: egui::Color32,
    pub width: f32,
}

pub struct AethravoxApp {
    pub rt: AsyncRt,
    pub tx: mpsc::UnboundedSender<ApiEvent>,
    pub rx: mpsc::UnboundedReceiver<ApiEvent>,

    // Conexión y sesión
    pub server_url: String,
    pub token: Option<String>,
    pub route: Route,
    pub offline_mode: bool,

    // Tema visual y dispositivo
    pub theme: ThemeKind,
    pub color_mode: ColorMode,
    pub device_profile: DeviceProfile,
    pub last_applied_profile: DeviceProfile,
    pub sidebar_collapsed: bool,
    pub mobile_menu_open: bool,
    pub mobile_detail_open: bool,
    pub fullscreen: bool,

    // Login / Register / Recover / Link state
    pub auth_tab_register: bool,
    pub auth_mode: u8, // 0=login, 1=register, 2=recover/reset, 3=link
    pub login_username: String,
    pub login_password: String,
    pub register_email: String,
    pub recover_email: String,
    pub reset_token: String,
    pub reset_new_password: String,
    pub link_code_input: String,
    pub link_device_name: String,
    pub auth_info: Option<String>,
    pub auth_error: Option<String>,
    pub auth_busy: bool,
    pub legal_modal: Option<&'static str>,

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
    pub chat_last_poll: Option<f64>,
    pub chat_new_panel: bool,
    pub chat_dm_username: String,
    pub chat_group_name: String,
    pub chat_group_desc: String,
    pub chat_group_members: String,
    pub chat_editing_id: Option<i64>,
    pub chat_edit_draft: String,
    pub chat_group_panel: bool,
    pub chat_group_manage: Option<ChatGroupManageDto>,
    pub chat_group_edit_name: String,
    pub chat_group_edit_desc: String,
    pub chat_group_add_user: String,
    pub chat_rank_name: String,
    pub chat_rank_color: String,
    pub chat_rank_can_send: bool,
    pub chat_rank_can_add: bool,
    pub chat_rank_can_edit_info: bool,
    pub chat_rank_can_delete: bool,
    pub chat_rank_can_remove: bool,
    pub chat_rank_can_manage: bool,
    pub chat_rank_can_assign: bool,
    pub chat_rank_can_pin: bool,
    pub chat_rank_can_transfer: bool,
    pub chat_attach_open: bool,
    pub chat_attach_is_image: bool,
    pub chat_attach_path: String,
    pub chat_attach_dropped_name: Option<String>,
    pub chat_attach_dropped_bytes: Option<Vec<u8>>,
    pub chat_attach_busy: bool,
    pub chat_select_mode: bool,
    pub chat_selected_ids: Vec<i64>,
    pub chat_msg_info: Option<ChatMessageInfoDto>,
    pub chat_group_img_path: String,
    pub chat_group_img_dropped_name: Option<String>,
    pub chat_group_img_dropped_bytes: Option<Vec<u8>>,

    // Aris state
    pub aris_chats: Vec<ArisChatDto>,
    pub active_aris_chat_id: i64,
    pub aris_messages: Vec<ArisMessageDto>,
    pub aris_mode: ArisMode,
    pub aris_draft: String,
    pub aris_credits: ArisCreditsDto,
    pub aris_busy: bool,
    pub aris_error: Option<String>,

    // Amigos state
    pub friends: FriendsPayload,
    pub friends_search_q: String,
    pub friends_search_results: Vec<UserSearchDto>,
    pub friends_direct_user: String,
    pub friends_error: Option<String>,

    // Comunidades state
    pub communities: Vec<CommunityDto>,
    pub active_community: Option<CommunityDetailDto>,
    pub community_inviteable: Vec<UserSearchDto>,
    pub community_invite_selected: Vec<i64>,
    pub community_new_name: String,
    pub community_new_desc: String,
    pub community_new_icon: String,
    pub community_group_name: String,
    pub community_group_desc: String,
    pub communities_error: Option<String>,

    // Documentos y Descargas state
    pub docs: Vec<DocumentDto>,
    pub docs_search: String,
    pub docs_error: Option<String>,
    pub downloads: Vec<DownloadPlatformDto>,
    pub downloads_error: Option<String>,

    // Asistencia / Tickets state
    pub tickets: Vec<TicketDto>,
    pub active_ticket: Option<TicketDetailDto>,
    pub ticket_type: String,
    pub ticket_subject: String,
    pub ticket_body: String,
    pub ticket_reply_draft: String,
    pub support_error: Option<String>,

    // Perfil y Ajustes state
    pub settings_tab_profile: bool,
    pub profile: Option<ProfilePayload>,
    pub profile_display_name: String,
    pub profile_bio: String,
    pub profile_selected_titles: Vec<i64>,
    pub profile_primary_title: Option<i64>,
    pub pw_current: String,
    pub pw_new: String,
    pub profile_error: Option<String>,
    pub link_code_generated: Option<LinkCodeDto>,
    pub emails_payload: Option<EmailsPayload>,
    pub email_input: String,
    pub email_pending_verify: Option<String>,
    pub email_verify_code: String,
    pub email_info: Option<String>,
    pub email_error: Option<String>,
    pub avatar_path: String,
    pub avatar_dropped_name: Option<String>,
    pub avatar_dropped_bytes: Option<Vec<u8>>,
    pub avatar_busy: bool,
    pub public_profile_modal: Option<PublicProfileDto>,
    pub public_profile_error: Option<String>,

    // aexTube state
    pub videos: Vec<VideoDto>,
    pub aextube_filter: String,
    pub aextube_error: Option<String>,
    pub aextube_upload_open: bool,
    pub aextube_up_title: String,
    pub aextube_up_desc: String,
    pub aextube_up_is_short: bool,
    pub aextube_up_path: String,
    pub aextube_up_busy: bool,
    pub aextube_up_dropped_name: Option<String>,
    pub aextube_up_dropped_bytes: Option<Vec<u8>>,

    // Playground state
    pub pg_tab: usize,
    pub pg_html: String,
    pub pg_css: String,
    pub pg_js: String,
    pub pg_prompt: String,
    pub pg_note: String,
    pub pg_busy: bool,
    pub pg_error: Option<String>,

    // aexOS state
    pub aexos_profiles: Vec<AexosProfileDto>,
    pub aexos_active_user_id: i64,
    pub aexos_link_open: bool,
    pub aexos_link_user: String,
    pub aexos_link_pass: String,
    pub aexos_switch_pass: String,
    pub aexos_switch_target: Option<i64>,
    pub aexos_error: Option<String>,
    pub aexos_new_file_name: String,

    // Transmitir state
    pub transmit_room: String,

    // Admin state
    pub admin_overview: Option<AdminOverviewDto>,
    pub admin_tab: usize,
    pub admin_ban_target: Option<(i64, String)>,
    pub admin_ban_reason: String,
    pub admin_ban_permanent: bool,
    pub admin_ban_hours: String,
    pub admin_suspend_target: Option<(i64, String)>,
    pub admin_suspend_reason: String,
    pub admin_suspend_hours: String,
    pub admin_delete_confirm: Option<(i64, String)>,
    pub admin_ip_bans: Option<AdminIpBansPayload>,
    pub admin_ip_ban_ip: String,
    pub admin_ip_ban_reason: String,
    pub admin_ip_ban_perm: bool,
    pub admin_ip_ban_hours: String,
    pub admin_config: Option<SystemConfigDto>,
    pub admin_cfg_studio: String,
    pub admin_cfg_date: String,
    pub admin_cfg_register: bool,
    pub admin_cfg_default_role: String,
    pub admin_cfg_maintenance: bool,
    pub admin_roles: Option<AdminRolesPayload>,
    pub admin_role_new_name: String,
    pub admin_role_new_color: String,
    pub admin_role_new_pos: String,
    pub admin_role_new_desc: String,
    pub admin_role_editing_id: Option<i64>,
    pub admin_role_edit_name: String,
    pub admin_role_edit_color: String,
    pub admin_role_edit_pos: String,
    pub admin_role_edit_desc: String,
    pub admin_role_selected_id: Option<i64>,
    pub admin_role_perms_draft: Vec<String>,
    pub admin_role_webs_draft: Vec<i64>,
    pub admin_role_delete_target: Option<(i64, String, i64)>,
    pub admin_role_reassign_id: Option<i64>,
    pub admin_titles: Option<AdminTitlesPayload>,
    pub admin_title_editing_id: Option<i64>,
    pub admin_title_name: String,
    pub admin_title_color: String,
    pub admin_title_effect: String,
    pub admin_title_desc: String,
    pub admin_title_assign_user: HashMap<i64, i64>,
    // Admin Users Create + IP History
    pub admin_new_user_open: bool,
    pub admin_new_user_name: String,
    pub admin_new_user_email: String,
    pub admin_new_user_pass: String,
    pub admin_new_user_role: String,
    pub admin_user_ips_target: Option<(i64, String)>,
    pub admin_user_ips_list: Vec<UserIpHistoryDto>,
    // Admin Webs Create + HTML + Tag
    pub admin_web_create_open: bool,
    pub admin_web_new_slug: String,
    pub admin_web_new_name: String,
    pub admin_web_new_desc: String,
    pub admin_web_new_icon: String,
    pub admin_web_new_type: String,
    pub admin_web_new_url: String,
    pub admin_web_new_html: String,
    pub admin_web_new_public: bool,
    pub admin_web_html_target: Option<(i64, String)>,
    pub admin_web_html_draft: String,
    pub admin_web_tag_target: Option<(i64, String)>,
    pub admin_web_tag_value: String,
    pub admin_web_tag_msg: String,
    pub admin_web_tag_opens: String,
    // Admin APIs Aris & Rotación
    pub admin_apis: Option<AdminApisPayload>,
    pub admin_apis_key_provider_id: i64,
    pub admin_apis_key_text: String,
    pub admin_apis_key_label: String,
    pub admin_apis_key_cap: String,
    pub admin_apis_key_caps_draft: HashMap<i64, String>,
    pub admin_apis_key_test_msg: HashMap<i64, (bool, String)>,
    pub admin_apis_prov_editing_id: Option<i64>,
    pub admin_apis_prov_name: String,
    pub admin_apis_prov_type: String,
    pub admin_apis_prov_url: String,
    pub admin_apis_prov_model: String,
    pub admin_apis_cfg_rot_enabled: bool,
    pub admin_apis_cfg_rot_strat: String,
    pub admin_apis_cfg_cooldown: String,
    pub admin_apis_cfg_failover: bool,
    pub admin_apis_cfg_ollama: String,
    pub admin_apis_cfg_hourly: String,
    pub admin_apis_cfg_daily: String,
    pub admin_apis_cfg_credits: String,
    pub admin_apis_cfg_mail_url: String,
    pub admin_apis_cfg_sys_prompt: String,
    // Admin Docs CRUD
    pub admin_docs: Option<Vec<DocumentDto>>,
    pub admin_doc_up_title: String,
    pub admin_doc_up_desc: String,
    pub admin_doc_up_path: String,
    pub admin_doc_up_dropped_name: Option<String>,
    pub admin_doc_up_dropped_bytes: Option<Vec<u8>>,
    pub admin_doc_edit_id: Option<i64>,
    pub admin_doc_edit_title: String,
    pub admin_doc_edit_desc: String,
    // Admin Downloads CRUD
    pub admin_downloads: Option<Vec<AdminDownloadRowDto>>,
    pub admin_dl_platform: String,
    pub admin_dl_slug: String,
    pub admin_dl_name: String,
    pub admin_dl_version: String,
    pub admin_dl_notes: String,
    pub admin_dl_path: String,
    pub admin_dl_dropped_name: Option<String>,
    pub admin_dl_dropped_bytes: Option<Vec<u8>>,
    pub admin_error: Option<String>,

    // Neon Canvas / PDI state
    pub neon_strokes: Vec<NeonStroke>,
    pub neon_active_stroke: Option<NeonStroke>,
    pub neon_color: egui::Color32,
    pub neon_width: f32,
    pub neon_board_mode: u8,

    // ZLibs (Zen Registry)
    pub zlibs_tab: u8,
    pub zlibs_packages: Vec<ZLibPackageDto>,
    pub zlibs_search: String,
    pub zlibs_active_detail: Option<ZLibPackageDetailDto>,
    pub zlibs_account: Option<ZLibAccountPayload>,
    pub zlibs_token_label: String,
    pub zlibs_created_token: Option<ZLibCreateTokenResponse>,
    pub zlibs_pub_name: String,
    pub zlibs_pub_version: String,
    pub zlibs_pub_desc: String,
    pub zlibs_pub_main: String,
    pub zlibs_pub_manifest: String,
    pub zlibs_pub_readme: String,
    pub zlibs_pub_files: Vec<ZLibFileInput>,
    pub zlibs_busy: bool,
    pub zlibs_error: Option<String>,
    pub zlibs_info: Option<String>,
    // Status toast
    pub status_banner: Option<String>,

    // Android IME & Teclado virtual integrado
    #[cfg(target_os = "android")]
    pub android_app: Option<winit::platform::android::activity::AndroidApp>,
    pub vk_manual_open: bool,
    pub vk_dismissed: bool,
    pub vk_shift: bool,
    pub vk_symbols: bool,
    pub vk_last_focus: Option<egui::Id>,
    pub vk_panel_rect: Option<egui::Rect>,
    pub was_wanting_keyboard: bool,
    pub vk_pending_events: Vec<egui::Event>,
}

impl AethravoxApp {
    #[allow(dead_code)]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        tx: mpsc::UnboundedSender<ApiEvent>,
        rx: mpsc::UnboundedReceiver<ApiEvent>,
        rt: AsyncRt,
    ) -> Self {
        Self::new_with_profile(cc, tx, rx, rt, DeviceProfile::Auto)
    }

    pub fn new_with_profile(
        cc: &eframe::CreationContext<'_>,
        tx: mpsc::UnboundedSender<ApiEvent>,
        rx: mpsc::UnboundedReceiver<ApiEvent>,
        rt: AsyncRt,
        initial_profile: DeviceProfile,
    ) -> Self {
        let theme = ThemeKind::Aethravox;
        let color_mode = ColorMode::Dark;
        let resolved = if initial_profile == DeviceProfile::Auto {
            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                DeviceProfile::Mobile
            }
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                DeviceProfile::Desktop
            }
        } else {
            initial_profile
        };
        apply_theme_for_device(&cc.egui_ctx, theme, color_mode, resolved);

        let (aris_chats, aris_messages) = demo_aris_chats();

        Self {
            rt,
            tx,
            rx,
            server_url: default_server_url(),
            token: None,
            route: Route::Login,
            offline_mode: false,
            theme,
            color_mode,
            device_profile: initial_profile,
            last_applied_profile: resolved,
            sidebar_collapsed: matches!(initial_profile, DeviceProfile::Tv),
            mobile_menu_open: false,
            mobile_detail_open: false,
            fullscreen: matches!(initial_profile, DeviceProfile::Tv | DeviceProfile::Pdi),
            auth_tab_register: false,
            auth_mode: 0,
            login_username: String::new(),
            login_password: String::new(),
            register_email: String::new(),
            recover_email: String::new(),
            reset_token: String::new(),
            reset_new_password: String::new(),
            link_code_input: String::new(),
            link_device_name: if cfg!(target_os = "android") {
                "Android".into()
            } else {
                "Desktop".into()
            },
            auth_info: None,
            auth_error: None,
            auth_busy: false,
            legal_modal: None,
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
            chat_new_panel: false,
            chat_dm_username: String::new(),
            chat_group_name: String::new(),
            chat_group_desc: String::new(),
            chat_group_members: String::new(),
            chat_editing_id: None,
            chat_edit_draft: String::new(),
            chat_group_panel: false,
            chat_group_manage: None,
            chat_group_edit_name: String::new(),
            chat_group_edit_desc: String::new(),
            chat_group_add_user: String::new(),
            chat_rank_name: String::new(),
            chat_rank_color: "#00d4ff".to_string(),
            chat_rank_can_send: true,
            chat_rank_can_add: false,
            chat_rank_can_edit_info: false,
            chat_rank_can_delete: false,
            chat_rank_can_remove: false,
            chat_rank_can_manage: false,
            chat_rank_can_assign: false,
            chat_rank_can_pin: false,
            chat_rank_can_transfer: false,
            chat_attach_open: false,
            chat_attach_is_image: true,
            chat_attach_path: String::new(),
            chat_attach_dropped_name: None,
            chat_attach_dropped_bytes: None,
            chat_attach_busy: false,
            chat_select_mode: false,
            chat_selected_ids: Vec::new(),
            chat_msg_info: None,
            chat_group_img_path: String::new(),
            chat_group_img_dropped_name: None,
            chat_group_img_dropped_bytes: None,
            aris_chats,
            active_aris_chat_id: 1,
            aris_messages,
            aris_mode: ArisMode::Instantaneo,
            aris_draft: String::new(),
            aris_credits: ArisCreditsDto {
                daily_limit: 120,
                daily_used: 0,
                remaining: 120,
            },
            aris_busy: false,
            aris_error: None,
            friends: demo_friends(),
            friends_search_q: String::new(),
            friends_search_results: Vec::new(),
            friends_direct_user: String::new(),
            friends_error: None,
            communities: demo_communities(),
            active_community: None,
            community_inviteable: Vec::new(),
            community_invite_selected: Vec::new(),
            community_new_name: String::new(),
            community_new_desc: String::new(),
            community_new_icon: String::new(),
            community_group_name: String::new(),
            community_group_desc: String::new(),
            communities_error: None,
            docs: demo_docs(),
            docs_search: String::new(),
            docs_error: None,
            downloads: demo_downloads(),
            downloads_error: None,
            tickets: demo_tickets(),
            active_ticket: None,
            ticket_type: "bug".to_string(),
            ticket_subject: String::new(),
            ticket_body: String::new(),
            ticket_reply_draft: String::new(),
            support_error: None,
            settings_tab_profile: false,
            profile: None,
            profile_display_name: String::new(),
            profile_bio: String::new(),
            profile_selected_titles: Vec::new(),
            profile_primary_title: None,
            pw_current: String::new(),
            pw_new: String::new(),
            profile_error: None,
            link_code_generated: None,
            emails_payload: None,
            email_input: String::new(),
            email_pending_verify: None,
            email_verify_code: String::new(),
            email_info: None,
            email_error: None,
            avatar_path: String::new(),
            avatar_dropped_name: None,
            avatar_dropped_bytes: None,
            avatar_busy: false,
            public_profile_modal: None,
            public_profile_error: None,
            videos: demo_videos(),
            aextube_filter: "all".to_string(),
            aextube_error: None,
            aextube_upload_open: false,
            aextube_up_title: String::new(),
            aextube_up_desc: String::new(),
            aextube_up_is_short: false,
            aextube_up_path: String::new(),
            aextube_up_busy: false,
            aextube_up_dropped_name: None,
            aextube_up_dropped_bytes: None,
            pg_tab: 0,
            pg_html: "<div class=\"hero\">\n  <h1>Hola Aethravox</h1>\n  <p>Edita este HTML, CSS o JS y ábrelo en vivo.</p>\n</div>".into(),
            pg_css: "body { background:#06030c; color:#f8f8fc; font-family:sans-serif; padding:40px; }\nh1 { color:#e0a308; }".into(),
            pg_js: "console.log('Playground listo');".into(),
            pg_prompt: String::new(),
            pg_note: String::new(),
            pg_busy: false,
            pg_error: None,
            aexos_profiles: demo_aexos_profiles("Aethravox"),
            aexos_active_user_id: 1,
            aexos_link_open: false,
            aexos_link_user: String::new(),
            aexos_link_pass: String::new(),
            aexos_switch_pass: String::new(),
            aexos_switch_target: None,
            aexos_error: None,
            aexos_new_file_name: String::new(),
            transmit_room: "estudio-01".to_string(),
            admin_overview: None,
            admin_tab: 0,
            admin_ban_target: None,
            admin_ban_reason: String::new(),
            admin_ban_permanent: false,
            admin_ban_hours: "24".to_string(),
            admin_suspend_target: None,
            admin_suspend_reason: String::new(),
            admin_suspend_hours: "24".to_string(),
            admin_delete_confirm: None,
            admin_ip_bans: None,
            admin_ip_ban_ip: String::new(),
            admin_ip_ban_reason: String::new(),
            admin_ip_ban_perm: false,
            admin_ip_ban_hours: "24".to_string(),
            admin_config: None,
            admin_cfg_studio: String::new(),
            admin_cfg_date: String::new(),
            admin_cfg_register: true,
            admin_cfg_default_role: "usuario".to_string(),
            admin_cfg_maintenance: false,
            admin_roles: None,
            admin_role_new_name: String::new(),
            admin_role_new_color: "#8b5cf6".to_string(),
            admin_role_new_pos: "150".to_string(),
            admin_role_new_desc: String::new(),
            admin_role_editing_id: None,
            admin_role_edit_name: String::new(),
            admin_role_edit_color: String::new(),
            admin_role_edit_pos: String::new(),
            admin_role_edit_desc: String::new(),
            admin_role_selected_id: None,
            admin_role_perms_draft: Vec::new(),
            admin_role_webs_draft: Vec::new(),
            admin_role_delete_target: None,
            admin_role_reassign_id: None,
            admin_titles: None,
            admin_title_editing_id: None,
            admin_title_name: String::new(),
            admin_title_color: "#e0a308".to_string(),
            admin_title_effect: "none".to_string(),
            admin_title_desc: String::new(),
            admin_title_assign_user: HashMap::new(),
            admin_new_user_open: false,
            admin_new_user_name: String::new(),
            admin_new_user_email: String::new(),
            admin_new_user_pass: String::new(),
            admin_new_user_role: "usuario".to_string(),
            admin_user_ips_target: None,
            admin_user_ips_list: Vec::new(),
            admin_web_create_open: false,
            admin_web_new_slug: String::new(),
            admin_web_new_name: String::new(),
            admin_web_new_desc: String::new(),
            admin_web_new_icon: "🧩".to_string(),
            admin_web_new_type: "external".to_string(),
            admin_web_new_url: String::new(),
            admin_web_new_html: String::new(),
            admin_web_new_public: false,
            admin_web_html_target: None,
            admin_web_html_draft: String::new(),
            admin_web_tag_target: None,
            admin_web_tag_value: "activo".to_string(),
            admin_web_tag_msg: String::new(),
            admin_web_tag_opens: String::new(),
            admin_apis: None,
            admin_apis_key_provider_id: 1,
            admin_apis_key_text: String::new(),
            admin_apis_key_label: String::new(),
            admin_apis_key_cap: String::new(),
            admin_apis_key_caps_draft: HashMap::new(),
            admin_apis_key_test_msg: HashMap::new(),
            admin_apis_prov_editing_id: None,
            admin_apis_prov_name: String::new(),
            admin_apis_prov_type: "openai_compat".to_string(),
            admin_apis_prov_url: String::new(),
            admin_apis_prov_model: String::new(),
            admin_apis_cfg_rot_enabled: true,
            admin_apis_cfg_rot_strat: "round_robin".to_string(),
            admin_apis_cfg_cooldown: "60".to_string(),
            admin_apis_cfg_failover: true,
            admin_apis_cfg_ollama: "http://127.0.0.1:11434".to_string(),
            admin_apis_cfg_hourly: "30".to_string(),
            admin_apis_cfg_daily: "150".to_string(),
            admin_apis_cfg_credits: "120".to_string(),
            admin_apis_cfg_mail_url: String::new(),
            admin_apis_cfg_sys_prompt: String::new(),
            admin_docs: None,
            admin_doc_up_title: String::new(),
            admin_doc_up_desc: String::new(),
            admin_doc_up_path: String::new(),
            admin_doc_up_dropped_name: None,
            admin_doc_up_dropped_bytes: None,
            admin_doc_edit_id: None,
            admin_doc_edit_title: String::new(),
            admin_doc_edit_desc: String::new(),
            admin_downloads: None,
            admin_dl_platform: "windows".to_string(),
            admin_dl_slug: String::new(),
            admin_dl_name: String::new(),
            admin_dl_version: "1.0.0".to_string(),
            admin_dl_notes: String::new(),
            admin_dl_path: String::new(),
            admin_dl_dropped_name: None,
            admin_dl_dropped_bytes: None,
            admin_error: None,
            neon_strokes: Vec::new(),
            neon_active_stroke: None,
            neon_color: egui::Color32::from_rgb(0, 212, 255),
            neon_width: 4.0,
            neon_board_mode: 0,
            zlibs_tab: 0,
            zlibs_packages: demo_zlibs(),
            zlibs_search: String::new(),
            zlibs_active_detail: None,
            zlibs_account: None,
            zlibs_token_label: String::new(),
            zlibs_created_token: None,
            zlibs_pub_name: String::new(),
            zlibs_pub_version: "1.0.0".to_string(),
            zlibs_pub_desc: String::new(),
            zlibs_pub_main: String::new(),
            zlibs_pub_manifest: String::new(),
            zlibs_pub_readme: String::new(),
            zlibs_pub_files: Vec::new(),
            zlibs_busy: false,
            zlibs_error: None,
            zlibs_info: None,
            status_banner: None,
            #[cfg(target_os = "android")]
            android_app: None,
            vk_manual_open: false,
            vk_dismissed: false,
            vk_shift: false,
            vk_symbols: false,
            vk_last_focus: None,
            vk_panel_rect: None,
            was_wanting_keyboard: false,
            vk_pending_events: Vec::new(),
        }
    }

    pub fn resolved_device(&self, ctx: &egui::Context) -> DeviceProfile {
        if self.device_profile != DeviceProfile::Auto {
            return self.device_profile;
        }
        let w = ctx.screen_rect().width();
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            if w > 0.0 && w < 900.0 {
                return DeviceProfile::Mobile;
            }
        }
        if w > 0.0 && w < 760.0 {
            DeviceProfile::Mobile
        } else {
            DeviceProfile::Desktop
        }
    }

    pub fn is_mobile_layout(&self, ctx: &egui::Context) -> bool {
        self.resolved_device(ctx) == DeviceProfile::Mobile || ctx.screen_rect().width() < 760.0
    }

    pub fn set_device_profile(&mut self, ctx: &egui::Context, profile: DeviceProfile) {
        self.device_profile = profile;
        let resolved = self.resolved_device(ctx);
        self.last_applied_profile = resolved;
        apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
        if matches!(resolved, DeviceProfile::Tv | DeviceProfile::Pdi) && !self.fullscreen {
            self.fullscreen = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
    }

    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
    }

    pub fn set_theme(&mut self, ctx: &egui::Context, theme: ThemeKind) {
        self.theme = theme;
        let resolved = self.resolved_device(ctx);
        apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
        if !self.offline_mode {
            spawn_save_theme(
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
        let resolved = self.resolved_device(ctx);
        apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
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
        let (ac, am) = demo_aris_chats();
        self.aris_chats = ac;
        self.aris_messages = am;
        self.active_aris_chat_id = 1;
        self.friends = demo_friends();
        self.communities = demo_communities();
        self.docs = demo_docs();
        self.downloads = demo_downloads();
        self.zlibs_packages = demo_zlibs();
        self.tickets = demo_tickets();
        self.videos = demo_videos();
        self.aexos_profiles = demo_aexos_profiles(username);
        self.aexos_active_user_id = 1;
        self.route = Route::Hub;
    }

    pub fn navigate_to(&mut self, ctx: &egui::Context, route: Route) {
        self.route = route;
        self.mobile_menu_open = false;
        self.mobile_detail_open = false;
        if self.offline_mode {
            return;
        }
        let has_tok = self.token.as_ref().map(|t| !t.is_empty()).unwrap_or(false);
        match route {
            Route::Hub => {
                spawn_fetch_hub(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Aris if has_tok => {
                spawn_aris_chats(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
                spawn_aris_credits(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Chat if has_tok => {
                spawn_fetch_conversations(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Friends if has_tok => {
                spawn_fetch_friends(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Communities => {
                spawn_fetch_communities(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Docs => {
                spawn_fetch_docs(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Downloads => {
                spawn_fetch_downloads(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::ZLibs => {
                spawn_fetch_zlibs_packages(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                    self.zlibs_search.clone(),
                );
                if has_tok {
                    spawn_fetch_zlibs_mine(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
            }
            Route::Support => {
                spawn_fetch_tickets(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::AexTube => {
                spawn_fetch_videos(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Aexos if has_tok => {
                spawn_aexos_profiles(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Settings if has_tok && self.settings_tab_profile => {
                spawn_fetch_profile(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
                spawn_fetch_emails(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
                spawn_fetch_zlibs_mine(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            Route::Admin if has_tok => {
                spawn_admin_overview(
                    &self.rt,
                    self.tx.clone(),
                    ctx.clone(),
                    self.server_url.clone(),
                    self.token.clone(),
                );
            }
            _ => {}
        }
    }

    pub fn open_app(&mut self, ctx: &egui::Context, app: &WebAppDto) {
        if !app.clickable {
            self.locked_modal = Some((app.name.clone(), app.lock_message.clone()));
            return;
        }
        let target = match app.slug.as_str() {
            "aris" => Route::Aris,
            "chat" => Route::Chat,
            "friends" => Route::Friends,
            "communities" => Route::Communities,
            "docs" => Route::Docs,
            "neon-canvas" => Route::NeonCanvas,
            "aextube" => Route::AexTube,
            "playground" => Route::Playground,
            "aexos" => Route::Aexos,
            "descargas" => Route::Downloads,
            "zlibs" => Route::ZLibs,
            "support" => Route::Support,
            "transmitir" => Route::Transmit,
            "dashboard" | "perfil" => Route::Settings,
            "admin" => Route::Admin,
            other => {
                self.status_banner = Some(format!("Sección /{other} en preparación."));
                Route::Hub
            }
        };
        self.navigate_to(ctx, target);
    }

    pub fn logout(&mut self) {
        if !self.offline_mode && self.token.is_some() {
            spawn_logout(&self.rt, self.server_url.clone(), self.token.clone());
        }
        self.token = None;
        self.hub = None;
        self.offline_mode = false;
        self.login_password.clear();
        self.profile = None;
        self.emails_payload = None;
        self.link_code_generated = None;
        self.admin_overview = None;
        self.zlibs_account = None;
        self.zlibs_created_token = None;
        self.zlibs_active_detail = None;
        self.admin_ip_bans = None;
        self.admin_config = None;
        self.admin_ban_target = None;
        self.admin_suspend_target = None;
        self.admin_delete_confirm = None;
        self.admin_roles = None;
        self.admin_titles = None;
        self.admin_role_editing_id = None;
        self.admin_role_selected_id = None;
        self.admin_role_delete_target = None;
        self.admin_title_editing_id = None;
        self.avatar_path.clear();
        self.avatar_dropped_name = None;
        self.avatar_dropped_bytes = None;
        self.avatar_busy = false;
        self.public_profile_modal = None;
        self.public_profile_error = None;
        self.aextube_upload_open = false;
        self.aextube_up_busy = false;
        self.aextube_up_dropped_name = None;
        self.aextube_up_dropped_bytes = None;
        self.active_community = None;
        self.active_ticket = None;
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
        self.chat_editing_id = None;
        self.chat_edit_draft.clear();
        self.chat_group_panel = false;
        self.chat_group_manage = None;
        self.chat_attach_open = false;
        self.chat_attach_path.clear();
        self.chat_attach_dropped_name = None;
        self.chat_attach_dropped_bytes = None;
        self.chat_attach_busy = false;
        self.chat_select_mode = false;
        self.chat_selected_ids.clear();
        self.chat_msg_info = None;
        self.chat_group_img_path.clear();
        self.chat_group_img_dropped_name = None;
        self.chat_group_img_dropped_bytes = None;
    }

    /// Abre una conversación: carga sus mensajes y la marca como leída (modo online).
    pub fn chat_open_conversation(&mut self, ctx: &egui::Context, conv_id: i64) {
        self.active_conv_id = conv_id;
        self.chat_error = None;
        self.chat_editing_id = None;
        self.chat_edit_draft.clear();
        self.chat_group_panel = false;
        self.chat_group_manage = None;
        self.chat_select_mode = false;
        self.chat_selected_ids.clear();
        self.chat_msg_info = None;
        self.chat_attach_open = false;
        self.chat_group_img_path.clear();
        self.chat_group_img_dropped_name = None;
        self.chat_group_img_dropped_bytes = None;
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

        let now = ctx.input(|i| i.time);
        let due = self
            .chat_last_poll
            .map(|t| (now - t) >= 3.0)
            .unwrap_or(true);

        if due {
            self.chat_last_poll = Some(now);
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
                    self.auth_info = None;
                    self.offline_mode = false;
                    self.token = Some(auth.token.clone());
                    self.reset_chat_state();
                    self.aris_chats.clear();
                    self.aris_messages.clear();
                    self.active_aris_chat_id = 0;
                    self.theme = auth.user.theme;
                    let resolved = self.resolved_device(ctx);
                    apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
                    self.route = Route::Hub;
                    spawn_fetch_hub(
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
                    let resolved = self.resolved_device(ctx);
                    apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
                    self.hub = Some(hub);
                }
                ApiEvent::HubErr(msg) => {
                    self.status_banner = Some(msg);
                }
                ApiEvent::ThemeSaved(_) => {}
                ApiEvent::StatusNote(note) => {
                    self.status_banner = Some(note);
                }

                // Chat
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
                    self.chat_attach_busy = false;
                    self.chat_pending = None;
                    self.chat_attach_open = false;
                    self.chat_attach_path.clear();
                    self.chat_attach_dropped_name = None;
                    self.chat_attach_dropped_bytes = None;
                    if !self.chat_messages.iter().any(|m| m.id == msg.id) {
                        self.chat_messages.push(msg);
                    }
                    self.chat_last_poll = None;
                }
                ApiEvent::ChatRoomOpened(conv_id) => {
                    self.chat_last_poll = None;
                    spawn_fetch_conversations(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                    self.chat_open_conversation(ctx, conv_id);
                }
                ApiEvent::ChatGroupManageLoaded(dto) => {
                    self.chat_error = None;
                    self.chat_group_edit_name = dto.name.clone();
                    self.chat_group_edit_desc = dto.description.clone();
                    self.chat_group_img_path.clear();
                    self.chat_group_img_dropped_name = None;
                    self.chat_group_img_dropped_bytes = None;
                    self.chat_group_manage = Some(dto);
                }
                ApiEvent::ChatGroupActionOk(msg) => {
                    self.chat_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_conversations(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::ChatGroupLeft(cid) => {
                    if self.active_conv_id == cid {
                        self.active_conv_id = 0;
                        self.chat_messages.retain(|m| m.conversation_id != cid);
                    }
                    self.chat_group_panel = false;
                    self.chat_group_manage = None;
                    self.mobile_detail_open = false;
                    self.status_banner = Some("Has salido del grupo.".into());
                    spawn_fetch_conversations(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::ChatMessageInfoLoaded(info) => {
                    self.chat_msg_info = Some(info);
                }
                ApiEvent::ChatBulkDeleted { ok, denied } => {
                    self.chat_select_mode = false;
                    self.chat_selected_ids.clear();
                    for m in &mut self.chat_messages {
                        if ok.contains(&m.id) {
                            m.content = "🚫 Mensaje eliminado".into();
                        }
                    }
                    self.status_banner = Some(if denied == 0 {
                        format!("{} mensaje(s) eliminado(s).", ok.len())
                    } else {
                        format!("{} eliminado(s), {} sin permiso.", ok.len(), denied)
                    });
                    if self.active_conv_id != 0 {
                        spawn_fetch_messages(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                            self.active_conv_id,
                        );
                    }
                }
                ApiEvent::ChatErr(msg) => {
                    self.chat_error = Some(msg);
                }
                ApiEvent::ChatSendErr(msg) => {
                    self.chat_sending = false;
                    self.chat_attach_busy = false;
                    self.chat_error = Some(msg);
                    if let Some(txt) = self.chat_pending.take() {
                        if self.chat_draft.is_empty() {
                            self.chat_draft = txt;
                        }
                    }
                }

                // Aris IA
                ApiEvent::ArisCreditsLoaded(c) => {
                    self.aris_credits = c;
                }
                ApiEvent::ArisChatsLoaded(list) => {
                    self.aris_error = None;
                    self.aris_chats = list;
                    let exists = self
                        .aris_chats
                        .iter()
                        .any(|c| c.id == self.active_aris_chat_id);
                    if !exists {
                        if let Some(first) = self.aris_chats.first().map(|c| c.id) {
                            self.active_aris_chat_id = first;
                            spawn_aris_messages(
                                &self.rt,
                                self.tx.clone(),
                                ctx.clone(),
                                self.server_url.clone(),
                                self.token.clone(),
                                first,
                            );
                        } else {
                            self.active_aris_chat_id = 0;
                            self.aris_messages.clear();
                        }
                    }
                }
                ApiEvent::ArisChatCreated(chat) => {
                    self.aris_error = None;
                    self.active_aris_chat_id = chat.id;
                    self.aris_chats.insert(0, chat);
                    self.aris_messages.clear();
                }
                ApiEvent::ArisChatDeleted(cid) => {
                    self.aris_chats.retain(|c| c.id != cid);
                    if self.active_aris_chat_id == cid {
                        self.active_aris_chat_id =
                            self.aris_chats.first().map(|c| c.id).unwrap_or(0);
                        self.aris_messages.clear();
                        if self.active_aris_chat_id > 0 {
                            spawn_aris_messages(
                                &self.rt,
                                self.tx.clone(),
                                ctx.clone(),
                                self.server_url.clone(),
                                self.token.clone(),
                                self.active_aris_chat_id,
                            );
                        }
                    }
                }
                ApiEvent::ArisChatPinned { chat_id, pinned } => {
                    if let Some(c) = self.aris_chats.iter_mut().find(|c| c.id == chat_id) {
                        c.pinned = pinned;
                    }
                }
                ApiEvent::ArisMessagesLoaded { chat_id, messages } => {
                    if chat_id == self.active_aris_chat_id {
                        self.aris_messages = messages;
                    }
                }
                ApiEvent::ArisReplyOk { chat_id, res } => {
                    self.aris_busy = false;
                    self.aris_error = None;
                    self.aris_credits = res.credits;
                    if chat_id == self.active_aris_chat_id {
                        self.aris_messages.push(res.assistant_message);
                        spawn_aris_messages(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                            chat_id,
                        );
                    }
                }
                ApiEvent::ArisErr(e) => {
                    self.aris_busy = false;
                    self.aris_error = Some(e);
                }

                // Amigos
                ApiEvent::FriendsLoaded(p) => {
                    self.friends_error = None;
                    self.friends = p;
                }
                ApiEvent::FriendSearchLoaded(list) => {
                    self.friends_search_results = list;
                }
                ApiEvent::FriendsActionOk(msg) => {
                    self.friends_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_friends(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::FriendsErr(e) => {
                    self.friends_error = Some(e);
                }

                // Comunidades
                ApiEvent::CommunitiesLoaded(list) => {
                    self.communities_error = None;
                    self.communities = list;
                }
                ApiEvent::CommunityDetailLoaded(detail) => {
                    self.communities_error = None;
                    self.active_community = Some(detail);
                }
                ApiEvent::CommunityInviteableLoaded(list) => {
                    self.community_inviteable = list;
                    self.community_invite_selected.clear();
                }
                ApiEvent::CommunitiesActionOk(msg) => {
                    self.communities_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_communities(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                    if let Some(cid) = self.active_community.as_ref().map(|d| d.community.id) {
                        spawn_community_detail(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                            cid,
                        );
                        spawn_community_inviteable(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                            cid,
                        );
                    }
                }
                ApiEvent::CommunitiesErr(e) => {
                    self.communities_error = Some(e);
                }

                // Docs y Downloads
                ApiEvent::DocsLoaded(list) => {
                    self.docs_error = None;
                    self.docs = list;
                }
                ApiEvent::DocsActionOk(msg) => {
                    self.docs_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_docs(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::DocsErr(e) => {
                    self.docs_error = Some(e);
                }
                ApiEvent::DownloadsLoaded(list) => {
                    self.downloads_error = None;
                    self.downloads = list;
                }
                ApiEvent::DownloadsErr(e) => {
                    self.downloads_error = Some(e);
                }

                // Support / Tickets
                ApiEvent::TicketsLoaded(list) => {
                    self.support_error = None;
                    self.tickets = list;
                }
                ApiEvent::TicketDetailLoaded(detail) => {
                    self.support_error = None;
                    self.active_ticket = Some(detail);
                }
                ApiEvent::SupportActionOk(msg) => {
                    self.support_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_tickets(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                    if let Some(tid) = self.active_ticket.as_ref().map(|d| d.ticket.id) {
                        spawn_ticket_detail(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                            tid,
                        );
                    }
                }
                ApiEvent::SupportErr(e) => {
                    self.support_error = Some(e);
                }

                // Perfil
                ApiEvent::ProfileLoaded(p) => {
                    self.profile_error = None;
                    self.profile_display_name = p.display_name.clone();
                    self.profile_bio = p.bio.clone();
                    self.profile_selected_titles = p
                        .titles
                        .iter()
                        .filter(|t| t.selected)
                        .map(|t| t.id)
                        .collect();
                    self.profile_primary_title =
                        p.titles.iter().find(|t| t.is_primary).map(|t| t.id);
                    self.profile = Some(p);
                }
                ApiEvent::ProfileActionOk(msg) => {
                    self.avatar_busy = false;
                    self.avatar_path.clear();
                    self.avatar_dropped_name = None;
                    self.avatar_dropped_bytes = None;
                    self.profile_error = None;
                    self.status_banner = Some(msg);
                    spawn_fetch_profile(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                    spawn_fetch_hub(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::ProfileErr(e) => {
                    self.avatar_busy = false;
                    self.profile_error = Some(e);
                }
                ApiEvent::PublicProfileLoaded(prof) => {
                    self.public_profile_error = None;
                    self.public_profile_modal = Some(prof);
                }
                ApiEvent::PublicProfileErr(e) => {
                    self.public_profile_error = Some(e.clone());
                    self.status_banner = Some(e);
                }

                // aexTube
                ApiEvent::VideosLoaded(list) => {
                    self.aextube_up_busy = false;
                    self.aextube_error = None;
                    self.videos = list;
                }
                ApiEvent::AextubeActionOk(msg) => {
                    self.aextube_up_busy = false;
                    self.aextube_error = None;
                    if msg.contains("publicado") {
                        self.aextube_up_title.clear();
                        self.aextube_up_desc.clear();
                        self.aextube_up_path.clear();
                        self.aextube_up_dropped_name = None;
                        self.aextube_up_dropped_bytes = None;
                        self.aextube_upload_open = false;
                    }
                    self.status_banner = Some(msg);
                    spawn_fetch_videos(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::AextubeErr(e) => {
                    self.aextube_up_busy = false;
                    self.aextube_error = Some(e);
                }

                // Playground
                ApiEvent::PlaygroundGenOk(res) => {
                    self.pg_busy = false;
                    self.pg_error = None;
                    self.pg_html = res.html;
                    self.pg_css = res.css;
                    self.pg_js = res.js;
                    self.pg_note = format!("{} ({} · {})", res.note, res.model, res.provider);
                }
                ApiEvent::PlaygroundGenErr(e) => {
                    self.pg_busy = false;
                    self.pg_error = Some(e);
                }

                // aexOS
                ApiEvent::AexosProfilesLoaded(list) => {
                    self.aexos_error = None;
                    let keep = list
                        .iter()
                        .any(|p| p.user_id == self.aexos_active_user_id);
                    if !keep {
                        self.aexos_active_user_id =
                            list.first().map(|p| p.user_id).unwrap_or(0);
                    }
                    self.aexos_profiles = list;
                }
                ApiEvent::AexosSwitched(uid) => {
                    self.aexos_error = None;
                    self.aexos_active_user_id = uid;
                    self.status_banner = Some("Perfil de aexOS cambiado.".into());
                }
                ApiEvent::AexosActionOk(msg) => {
                    self.aexos_error = None;
                    self.status_banner = Some(msg);
                }
                ApiEvent::AexosErr(e) => {
                    self.aexos_error = Some(e);
                }

                // Admin
                ApiEvent::AdminOverviewLoaded(ov) => {
                    self.admin_error = None;
                    self.admin_overview = Some(ov);
                }
                ApiEvent::AdminIpBansLoaded(p) => {
                    self.admin_error = None;
                    self.admin_ip_bans = Some(p);
                }
                ApiEvent::AdminConfigLoaded(cfg) => {
                    self.admin_error = None;
                    self.admin_cfg_studio = cfg.studio_name.clone();
                    self.admin_cfg_date = cfg.foundation_date.clone();
                    self.admin_cfg_register = cfg.allow_public_register;
                    self.admin_cfg_default_role = cfg.default_role.clone();
                    self.admin_cfg_maintenance = cfg.maintenance_mode;
                    self.admin_config = Some(cfg);
                }
                ApiEvent::AdminRolesLoaded(p) => {
                    self.admin_error = None;
                    let sel_id = self
                        .admin_role_selected_id
                        .filter(|id| p.roles.iter().any(|r| r.id == *id))
                        .or_else(|| p.roles.first().map(|r| r.id));
                    self.admin_role_selected_id = sel_id;
                    if let Some(sid) = sel_id {
                        if let Some(r) = p.roles.iter().find(|r| r.id == sid) {
                            self.admin_role_perms_draft = r.perms.clone();
                            self.admin_role_webs_draft = r.web_ids.clone();
                        }
                    } else {
                        self.admin_role_perms_draft.clear();
                        self.admin_role_webs_draft.clear();
                    }
                    self.admin_roles = Some(p);
                }
                ApiEvent::AdminTitlesLoaded(p) => {
                    self.admin_error = None;
                    self.admin_titles = Some(p);
                }
                ApiEvent::AdminUserIpsLoaded { user_id, ips } => {
                    self.admin_error = None;
                    if let Some((tid, _)) = &self.admin_user_ips_target {
                        if *tid == user_id {
                            self.admin_user_ips_list = ips;
                        }
                    }
                }
                ApiEvent::AdminApisLoaded(p) => {
                    self.admin_error = None;
                    if self.admin_apis_key_provider_id == 0
                        || !p.providers.iter().any(|pr| pr.id == self.admin_apis_key_provider_id)
                    {
                        if let Some(first) = p.providers.first() {
                            self.admin_apis_key_provider_id = first.id;
                        }
                    }
                    for k in &p.keys {
                        self.admin_apis_key_caps_draft.entry(k.id).or_insert_with(|| {
                            k.daily_cap.map(|c| c.to_string()).unwrap_or_default()
                        });
                    }
                    self.admin_apis_cfg_rot_enabled = p.config.rotation_enabled;
                    self.admin_apis_cfg_rot_strat = p.config.rotation_strategy.clone();
                    self.admin_apis_cfg_cooldown = p.config.cooldown_minutes.to_string();
                    self.admin_apis_cfg_failover = p.config.auto_failover;
                    self.admin_apis_cfg_ollama = p.config.ollama_url.clone();
                    self.admin_apis_cfg_hourly = p.config.hourly_limit.to_string();
                    self.admin_apis_cfg_daily = p.config.daily_limit.to_string();
                    self.admin_apis_cfg_credits = p.config.daily_credits_limit.to_string();
                    self.admin_apis_cfg_mail_url = p.config.aethramail_url.clone();
                    self.admin_apis_cfg_sys_prompt = p.config.system_prompt.clone();
                    self.admin_apis = Some(p);
                }
                ApiEvent::AdminApiKeyTested { key_id, res } => {
                    self.admin_error = None;
                    let msg = if res.ok {
                        format!(
                            "{} ({} ms): {}",
                            if res.healthy { "OK" } else { "Fallo" },
                            res.latency_ms,
                            res.detail
                        )
                    } else {
                        res.error.unwrap_or_else(|| "Error desconocido".into())
                    };
                    self.admin_apis_key_test_msg
                        .insert(key_id, (res.ok && res.healthy, msg));
                }
                ApiEvent::AdminDocsLoaded(docs) => {
                    self.admin_error = None;
                    self.admin_docs = Some(docs.clone());
                    self.docs = docs;
                }
                ApiEvent::AdminDownloadsLoaded(rows) => {
                    self.admin_error = None;
                    self.admin_downloads = Some(rows);
                }
                ApiEvent::AdminActionOk(msg) => {
                    self.admin_error = None;
                    self.status_banner = Some(msg);
                    spawn_admin_overview(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                    if self.admin_tab == 3 {
                        spawn_admin_ip_bans(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 4 {
                        spawn_admin_config_get(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 5 {
                        spawn_admin_roles(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 6 {
                        spawn_admin_titles(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 7 {
                        spawn_admin_apis(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 8 {
                        spawn_admin_docs_list(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    } else if self.admin_tab == 9 {
                        spawn_admin_downloads_list(
                            &self.rt,
                            self.tx.clone(),
                            ctx.clone(),
                            self.server_url.clone(),
                            self.token.clone(),
                        );
                    }
                }
                ApiEvent::AdminErr(e) => {
                    self.admin_error = Some(e);
                }

                // Recuperación, Vinculación y Correo interno
                ApiEvent::RecoverOk(res) => {
                    self.auth_busy = false;
                    self.auth_error = None;
                    self.auth_info = Some(res.info);
                    if let Some(tok) = res.dev_token {
                        self.reset_token = tok;
                    }
                }
                ApiEvent::ResetOk => {
                    self.auth_busy = false;
                    self.auth_error = None;
                    self.auth_info = Some("Contraseña restablecida. Ya puedes iniciar sesión.".into());
                    self.reset_token.clear();
                    self.reset_new_password.clear();
                    self.auth_mode = 0;
                    self.auth_tab_register = false;
                }
                ApiEvent::LinkCodeLoaded(dto) => {
                    self.profile_error = None;
                    self.link_code_generated = Some(dto);
                }
                ApiEvent::EmailsLoaded(p) => {
                    self.email_error = None;
                    self.emails_payload = Some(p);
                    self.email_pending_verify = None;
                    self.email_verify_code.clear();
                }
                ApiEvent::EmailRequestOk(msg) => {
                    self.email_error = None;
                    self.email_info = Some(msg);
                    self.email_pending_verify = Some(self.email_input.trim().to_string());
                }
                ApiEvent::EmailErr(e) => {
                    self.email_error = Some(e);
                }
                ApiEvent::LogoutAllOk => {
                    self.logout();
                    self.auth_info = Some("Todas tus sesiones fueron cerradas.".into());
                }
                // ZLibs (Zen Registry)
                ApiEvent::ZLibsPackagesLoaded(list) => {
                    self.zlibs_error = None;
                    self.zlibs_packages = list;
                }
                ApiEvent::ZLibsDetailLoaded(detail) => {
                    self.zlibs_error = None;
                    self.zlibs_active_detail = Some(detail);
                }
                ApiEvent::ZLibsAccountLoaded(acc) => {
                    self.zlibs_error = None;
                    self.zlibs_account = Some(acc);
                }
                ApiEvent::ZLibTokenCreated(res) => {
                    self.zlibs_error = None;
                    self.zlibs_info = Some("Token CLI generado correctamente.".into());
                    self.zlibs_token_label.clear();
                    self.zlibs_created_token = Some(res);
                    spawn_fetch_zlibs_mine(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::ZLibPublished(res) => {
                    self.zlibs_busy = false;
                    self.zlibs_error = None;
                    self.zlibs_info = Some(format!(
                        "Librería publicada en {} (ref {}).",
                        res.git_url, res.git_ref
                    ));
                    self.status_banner = Some("Paquete Zen publicado en GitHub + ZLibs.".into());
                    self.zlibs_pub_files.clear();
                    self.zlibs_tab = 0;
                    spawn_fetch_zlibs_packages(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                        self.zlibs_search.clone(),
                    );
                    spawn_fetch_zlibs_mine(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                    );
                }
                ApiEvent::ZLibsErr(e) => {
                    self.zlibs_busy = false;
                    self.zlibs_error = Some(e);
                }
            }
        }
    }
}

impl eframe::App for AethravoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.vk_pending_events.is_empty() {
            if let Some(fid) = self.vk_last_focus {
                ctx.memory_mut(|m| m.request_focus(fid));
            }
            let pending = std::mem::take(&mut self.vk_pending_events);
            ctx.input_mut(|i| i.events.extend(pending));
        }
        self.drain_events(ctx);
        self.chat_poll(ctx);

        let resolved = self.resolved_device(ctx);
        if resolved != self.last_applied_profile {
            self.last_applied_profile = resolved;
            apply_theme_for_device(ctx, self.theme, self.color_mode, resolved);
        }

        // Navegación rápida atrás (Esc en TV/Android Back o PDI)
        if self.route != Route::Login
            && self.route != Route::Hub
            && matches!(resolved, DeviceProfile::Tv | DeviceProfile::Mobile)
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            if self.mobile_menu_open {
                self.mobile_menu_open = false;
            } else if self.mobile_detail_open {
                self.mobile_detail_open = false;
            } else {
                self.navigate_to(ctx, Route::Hub);
            }
        }

        let p: Palette = Palette::from(self.theme, self.color_mode);

        // Gestión del teclado Android / IME + teclado virtual integrado
        let wants_kb = ctx.wants_keyboard_input();
        let pointer_in_vk = self
            .vk_panel_rect
            .map(|r| {
                ctx.input(|i| {
                    i.pointer
                        .latest_pos()
                        .map(|pos| r.contains(pos))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if wants_kb {
            if let Some(fid) = ctx.memory(|m| m.focused()) {
                self.vk_last_focus = Some(fid);
            }
            if !self.was_wanting_keyboard {
                self.vk_dismissed = false;
                #[cfg(target_os = "android")]
                if let Some(aa) = &self.android_app {
                    aa.show_soft_input(true);
                }
            }
        } else if !pointer_in_vk && self.was_wanting_keyboard {
            self.vk_dismissed = false;
            #[cfg(target_os = "android")]
            if let Some(aa) = &self.android_app {
                aa.hide_soft_input(true);
            }
        }
        self.was_wanting_keyboard = wants_kb;

        let show_vk = self.vk_manual_open
            || ((cfg!(target_os = "android") || resolved == DeviceProfile::Pdi)
                && !self.vk_dismissed
                && (wants_kb || (pointer_in_vk && self.vk_last_focus.is_some())));

        if show_vk {
            ui::virtual_keyboard::show(self, ctx, &p);
        } else {
            self.vk_panel_rect = None;
        }

        if self.route == Route::Login {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(p.bg_base))
                .show(ctx, |ui| {
                    ui::login::show(self, ui, ctx, &p);
                });
            return;
        }

        let mobile = self.is_mobile_layout(ctx);
        let safe_top = if cfg!(any(target_os = "android", target_os = "ios")) {
            14.0
        } else {
            8.0
        };
        let top_h = if mobile {
            58.0 + safe_top
        } else if matches!(resolved, DeviceProfile::Tv | DeviceProfile::Pdi) {
            62.0
        } else {
            54.0
        };

        // Top header bar
        egui::TopBottomPanel::top("top_header")
            .exact_height(top_h)
            .frame(
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                    .inner_margin(egui::Margin {
                        left: 14.0,
                        right: 14.0,
                        top: safe_top,
                        bottom: 8.0,
                    }),
            )
            .show(ctx, |ui| {
                ui::sidebar::show_topbar(self, ui, ctx, &p);
            });

        if mobile && !show_vk {
            let safe_bottom = if cfg!(any(target_os = "android", target_os = "ios")) {
                14.0
            } else {
                6.0
            };
            egui::TopBottomPanel::bottom("mobile_nav")
                .exact_height(56.0 + safe_bottom)
                .frame(
                    egui::Frame::none()
                        .fill(p.bg_surface)
                        .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                        .inner_margin(egui::Margin {
                            left: 8.0,
                            right: 8.0,
                            top: 6.0,
                            bottom: safe_bottom,
                        }),
                )
                .show(ctx, |ui| {
                    ui::sidebar::show_mobile_bottom_bar(self, ui, ctx, &p);
                });
        } else {
            // Left navigation sidebar (PC / TV / PDI)
            let side_w = if self.sidebar_collapsed {
                68.0
            } else if matches!(resolved, DeviceProfile::Tv | DeviceProfile::Pdi) {
                268.0
            } else {
                228.0
            };
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
        }

        if resolved == DeviceProfile::Tv {
            egui::TopBottomPanel::bottom("tv_hint_bar")
                .exact_height(32.0)
                .frame(
                    egui::Frame::none()
                        .fill(p.bg_surface)
                        .stroke(egui::Stroke::new(1.0_f32, p.border_subtle))
                        .inner_margin(egui::Margin::symmetric(16.0, 4.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "📺 Modo TV · Usa cruceta / Tab + Enter para seleccionar · Esc / Atrás vuelve al Hub",
                            )
                            .small()
                            .color(p.text_muted),
                        );
                    });
                });
        }

        let content_margin = if mobile { 10.0 } else { 22.0 };
        // Main workspace panel
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_base)
                    .inner_margin(egui::Margin::same(content_margin)),
            )
            .show(ctx, |ui| {
                if mobile && self.mobile_menu_open {
                    ui::sidebar::show_mobile_drawer(self, ui, ctx, &p);
                    return;
                }
                match self.route {
                    Route::Login => {}
                    Route::Hub => ui::hub::show(self, ui, &p),
                    Route::Aris => ui::aris::show(self, ui, &p),
                    Route::Chat => ui::chat::show(self, ui, &p),
                    Route::Friends => ui::friends::show(self, ui, &p),
                    Route::Communities => ui::communities::show(self, ui, &p),
                    Route::Docs => ui::docs::show(self, ui, &p),
                    Route::NeonCanvas => ui::neon_canvas::show(self, ui, &p),
                    Route::AexTube => ui::aextube::show(self, ui, &p),
                    Route::Playground => ui::playground::show(self, ui, &p),
                    Route::Aexos => ui::aexos::show(self, ui, &p),
                    Route::Downloads => ui::downloads::show(self, ui, &p),
                    Route::ZLibs => ui::zlibs::show(self, ui, &p),
                    Route::Support => ui::support::show(self, ui, &p),
                    Route::Transmit => ui::transmit::show(self, ui, &p),
                    Route::Settings => ui::settings::show(self, ui, ctx, &p),
                    Route::Admin => ui::admin::show(self, ui, &p),
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

        // Public Profile Modal
        if let Some(prof) = self.public_profile_modal.clone() {
            let mut open = true;
            let mut start_dm: Option<String> = None;
            let mut add_friend: Option<String> = None;
            let mut open_avatar: Option<String> = None;
            egui::Window::new(format!("👤 Perfil de @{}", prof.username))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .max_width(420.0)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&prof.display).heading().strong());
                        let rcol = parse_hex_color(&prof.role_color, p.accent_secondary);
                        badge_pill(
                            ui,
                            &prof.role_display,
                            rcol.gamma_multiply(0.22),
                            rcol,
                        );
                    });
                    ui.label(
                        egui::RichText::new(format!("@{} · Miembro desde {}", prof.username, prof.created_at))
                            .small()
                            .color(p.text_muted),
                    );
                    if !prof.avatar_url.is_empty() {
                        ui.add_space(4.0);
                        if ui.button("🖼 Ver avatar en navegador ↗").clicked() {
                            open_avatar = Some(format!(
                                "{}{}",
                                self.server_url.trim_end_matches('/'),
                                prof.avatar_url
                            ));
                        }
                    }
                    if let Some(pt) = &prof.primary_title {
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new("Título principal:").small().color(p.text_muted));
                            badge_pill(
                                ui,
                                &format!("★ {}", pt.name),
                                p.accent_primary.gamma_multiply(0.22),
                                p.accent_primary,
                            );
                        });
                    }
                    if !prof.visible_titles.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for t in &prof.visible_titles {
                                badge_pill(
                                    ui,
                                    &t.name,
                                    p.accent_secondary.gamma_multiply(0.16),
                                    p.accent_secondary,
                                );
                            }
                        });
                    }
                    if !prof.bio.trim().is_empty() {
                        ui.add_space(8.0);
                        ui.label(&prof.bio);
                    }
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        if !prof.is_self && primary_button(ui, &p, "💬 Enviar DM").clicked() {
                            start_dm = Some(prof.username.clone());
                        }
                        if !prof.is_self && !prof.is_friend && ui.button("➕ Añadir amigo").clicked() {
                            add_friend = Some(prof.username.clone());
                        }
                        if ui.button("Cerrar").clicked() {
                            self.public_profile_modal = None;
                        }
                    });
                });
            if !open {
                self.public_profile_modal = None;
            }
            if let Some(url) = open_avatar {
                ctx.open_url(egui::OpenUrl::new_tab(url));
            }
            if let Some(uname) = start_dm {
                self.public_profile_modal = None;
                if !self.offline_mode && self.token.is_some() {
                    spawn_start_dm(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                        uname,
                    );
                }
                self.navigate_to(ctx, Route::Chat);
            }
            if let Some(uname) = add_friend {
                if !self.offline_mode && self.token.is_some() {
                    spawn_send_friend_request(
                        &self.rt,
                        self.tx.clone(),
                        ctx.clone(),
                        self.server_url.clone(),
                        self.token.clone(),
                        uname,
                    );
                }
                self.public_profile_modal = None;
            }
        }

        // Chat Message Info Modal
        if let Some(info) = self.chat_msg_info.clone() {
            let mut open = true;
            egui::Window::new(format!("ℹ Información del mensaje #{}", info.id))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .max_width(420.0)
                .show(ctx, |ui| {
                    ui.label(format!("Remitente: {}", info.sender_name));
                    ui.label(format!("Conversación: #{}", info.conversation_id));
                    ui.label(format!("Enviado: {}", info.created_at));
                    ui.label(format!("Estado: {}", info.status));
                    if let Some(ed) = &info.edited_at {
                        ui.label(format!("Editado: {ed}"));
                    }
                    if let Some(del) = &info.deleted_at {
                        ui.label(format!("Eliminado: {del}"));
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(&info.body).small().color(p.text_muted));
                    ui.add_space(8.0);
                    if primary_button(ui, &p, "Cerrar").clicked() {
                        self.chat_msg_info = None;
                    }
                });
            if !open {
                self.chat_msg_info = None;
            }
        }
    }
}

fn default_server_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(origin) = web_sys::window().and_then(|w| w.location().origin().ok()) {
            if !origin.is_empty() && origin != "null" {
                return origin;
            }
        }
    }
    "https://aethravox-studios.duckdns.org".to_string()
}
