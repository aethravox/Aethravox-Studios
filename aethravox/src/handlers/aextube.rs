//! aexTube — Plataforma de vídeo de Aethravox Studios.
//!
//! - Lista vídeos y Shorts con reacciones (like/dislike), visualizaciones y suscripciones por canal.
//! - Subida de clips MP4 / WebM / MOV / OGG de hasta 20 MB guardados en `data/aextube/`.
//! - Servidos estáticamente bajo `/aextube/media/<filename>`.

use axum::{
    extract::{Multipart, Path, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{auth_mw::AuthUser, AppState};

const MAX_VIDEO_BYTES: usize = 20 * 1024 * 1024;
const MEDIA_DIR: &str = "data/aextube";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AextubeVideoRow {
    pub id: i64,
    pub user_id: i64,
    pub author: String,
    pub title: String,
    pub description: String,
    pub is_short: bool,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub likes: i64,
    pub dislikes: i64,
    pub views: i64,
    pub created_at: String,
    pub user_reaction: Option<String>,
    pub subscribed: bool,
    pub channel_subscribers: i64,
}

#[derive(serde::Deserialize)]
pub struct ReactPayload {
    pub kind: String,
}

pub async fn list_videos_for_user(
    state: &AppState,
    user_id: i64,
) -> Vec<AextubeVideoRow> {
    sqlx::query_as::<_, AextubeVideoRow>(
        "SELECT v.id, v.user_id, v.author, v.title, v.description, v.is_short,
                v.file_path, v.mime_type, v.size_bytes, v.likes, v.dislikes, v.views,
                strftime('%Y-%m-%d %H:%M', v.created_at) AS created_at,
                (SELECT r.reaction FROM aextube_reactions r WHERE r.video_id = v.id AND r.user_id = ?) AS user_reaction,
                EXISTS(SELECT 1 FROM aextube_subscriptions s WHERE s.channel_author = v.author AND s.user_id = ?) AS subscribed,
                (SELECT COUNT(*) FROM aextube_subscriptions s2 WHERE s2.channel_author = v.author) AS channel_subscribers
         FROM aextube_videos v
         ORDER BY v.id DESC
         LIMIT 100",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(&state.db.pool)
    .await
    .unwrap_or_default()
}

pub async fn aextube_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let videos = list_videos_for_user(&state, user.id).await;
    let videos_json = serde_json::to_string(&videos)
        .unwrap_or_else(|_| "[]".to_string())
        .replace('<', "\\u003c");
    let can_moderate = state.db.user_has_perm(&user, "admin.access").await;
    let can_use_aris_studio = state.db.user_has_perm(&user, "aris.lane.studio").await;
    crate::templates::AextubeTemplate {
        username: user.username.clone(),
        display: user.display(),
        role: user.role.clone(),
        user_id: user.id,
        can_moderate,
        can_use_aris_studio,
        videos_json,
    }
}

pub async fn api_list_videos(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let videos = list_videos_for_user(&state, user.id).await;
    Json(serde_json::json!({ "ok": true, "videos": videos })).into_response()
}

pub async fn api_upload_video(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> Response {
    let mut title = String::new();
    let mut description = String::new();
    let mut is_short = false;
    let mut video_bytes: Option<Vec<u8>> = None;
    let mut content_type = "video/mp4".to_string();
    let mut orig_name = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "title" => {
                title = field.text().await.unwrap_or_default().trim().to_string();
            }
            "description" => {
                description = field.text().await.unwrap_or_default().trim().to_string();
            }
            "is_short" => {
                let v = field.text().await.unwrap_or_default();
                is_short = v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on");
            }
            "video" => {
                orig_name = field.file_name().unwrap_or("clip.mp4").to_string();
                content_type = field.content_type().unwrap_or("video/mp4").to_string();
                match field.bytes().await {
                    Ok(b) => {
                        if b.len() > MAX_VIDEO_BYTES {
                            return Json(serde_json::json!({
                                "error": "El archivo supera el límite de 20 MB."
                            })).into_response();
                        }
                        video_bytes = Some(b.to_vec());
                    }
                    Err(_) => {
                        return Json(serde_json::json!({
                            "error": "Error al leer el archivo de vídeo."
                        })).into_response();
                    }
                }
            }
            _ => {}
        }
    }

    if title.is_empty() {
        return Json(serde_json::json!({ "error": "El título es obligatorio." })).into_response();
    }
    let Some(bytes) = video_bytes else {
        return Json(serde_json::json!({ "error": "Debes adjuntar un archivo de vídeo." })).into_response();
    };
    if bytes.is_empty() {
        return Json(serde_json::json!({ "error": "El archivo está vacío." })).into_response();
    }

    let lower_name = orig_name.to_lowercase();
    let ext = if content_type.contains("webm") || lower_name.ends_with(".webm") {
        "webm"
    } else if content_type.contains("quicktime") || lower_name.ends_with(".mov") {
        "mov"
    } else if content_type.contains("ogg") || lower_name.ends_with(".ogv") || lower_name.ends_with(".ogg") {
        "ogv"
    } else {
        "mp4"
    };

    if !content_type.starts_with("video/") && !["mp4", "webm", "mov", "ogv"].contains(&ext) {
        return Json(serde_json::json!({ "error": "Formato de vídeo no soportado." })).into_response();
    }

    if std::fs::create_dir_all(MEDIA_DIR).is_err() {
        return Json(serde_json::json!({ "error": "No se pudo preparar la carpeta multimedia." })).into_response();
    }

    let filename = format!("{}.{}", crate::db::random_hex(12), ext);
    let disk_path = format!("{}/{}", MEDIA_DIR, filename);
    if std::fs::write(&disk_path, &bytes).is_err() {
        return Json(serde_json::json!({ "error": "No se pudo guardar el vídeo en el servidor." })).into_response();
    }

    let public_url = format!("/aextube/media/{}", filename);
    let author = user.display();
    let size_bytes = bytes.len() as i64;

    let res = sqlx::query_scalar::<_, i64>(
        "INSERT INTO aextube_videos (user_id, author, title, description, is_short, file_path, mime_type, size_bytes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(user.id)
    .bind(&author)
    .bind(&title)
    .bind(&description)
    .bind(is_short as i64)
    .bind(&public_url)
    .bind(&content_type)
    .bind(size_bytes)
    .fetch_one(&state.db.pool)
    .await;

    match res {
        Ok(id) => {
            let videos = list_videos_for_user(&state, user.id).await;
            Json(serde_json::json!({ "ok": true, "video_id": id, "videos": videos })).into_response()
        }
        Err(_) => {
            let _ = std::fs::remove_file(&disk_path);
            Json(serde_json::json!({ "error": "Error al registrar el vídeo en la base de datos." })).into_response()
        }
    }
}

pub async fn api_react_video(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(video_id): Path<i64>,
    Json(payload): Json<ReactPayload>,
) -> Response {
    if payload.kind != "like" && payload.kind != "dislike" {
        return Json(serde_json::json!({ "error": "reaccion_invalida" })).into_response();
    }

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT reaction FROM aextube_reactions WHERE video_id = ? AND user_id = ?",
    )
    .bind(video_id)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await
    .unwrap_or(None);

    let new_reaction: Option<String>;
    if let Some((prev,)) = existing {
        if prev == payload.kind {
            let _ = sqlx::query("DELETE FROM aextube_reactions WHERE video_id = ? AND user_id = ?")
                .bind(video_id)
                .bind(user.id)
                .execute(&state.db.pool)
                .await;
            new_reaction = None;
        } else {
            let _ = sqlx::query("UPDATE aextube_reactions SET reaction = ? WHERE video_id = ? AND user_id = ?")
                .bind(&payload.kind)
                .bind(video_id)
                .bind(user.id)
                .execute(&state.db.pool)
                .await;
            new_reaction = Some(payload.kind);
        }
    } else {
        let _ = sqlx::query(
            "INSERT INTO aextube_reactions (video_id, user_id, reaction) VALUES (?, ?, ?)",
        )
        .bind(video_id)
        .bind(user.id)
        .bind(&payload.kind)
        .execute(&state.db.pool)
        .await;
        new_reaction = Some(payload.kind);
    }

    let likes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aextube_reactions WHERE video_id = ? AND reaction = 'like'",
    )
    .bind(video_id)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);
    let dislikes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aextube_reactions WHERE video_id = ? AND reaction = 'dislike'",
    )
    .bind(video_id)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    let _ = sqlx::query("UPDATE aextube_videos SET likes = ?, dislikes = ? WHERE id = ?")
        .bind(likes)
        .bind(dislikes)
        .bind(video_id)
        .execute(&state.db.pool)
        .await;

    Json(serde_json::json!({
        "ok": true,
        "likes": likes,
        "dislikes": dislikes,
        "user_reaction": new_reaction,
    }))
    .into_response()
}

pub async fn api_view_video(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(video_id): Path<i64>,
) -> Response {
    let _ = sqlx::query("UPDATE aextube_videos SET views = views + 1 WHERE id = ?")
        .bind(video_id)
        .execute(&state.db.pool)
        .await;
    let views: i64 = sqlx::query_scalar("SELECT views FROM aextube_videos WHERE id = ?")
        .bind(video_id)
        .fetch_one(&state.db.pool)
        .await
        .unwrap_or(0);
    Json(serde_json::json!({ "ok": true, "views": views })).into_response()
}

pub async fn api_toggle_subscribe(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(author): Path<String>,
) -> Response {
    let clean_author = author.trim().to_string();
    if clean_author.is_empty() {
        return Json(serde_json::json!({ "error": "autor_invalido" })).into_response();
    }

    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM aextube_subscriptions WHERE channel_author = ? AND user_id = ?",
    )
    .bind(&clean_author)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await
    .unwrap_or(None);

    let subscribed = if exists.is_some() {
        let _ = sqlx::query("DELETE FROM aextube_subscriptions WHERE channel_author = ? AND user_id = ?")
            .bind(&clean_author)
            .bind(user.id)
            .execute(&state.db.pool)
            .await;
        false
    } else {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO aextube_subscriptions (channel_author, user_id) VALUES (?, ?)",
        )
        .bind(&clean_author)
        .bind(user.id)
        .execute(&state.db.pool)
        .await;
        true
    };

    let subscribers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aextube_subscriptions WHERE channel_author = ?",
    )
    .bind(&clean_author)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);

    Json(serde_json::json!({
        "ok": true,
        "author": clean_author,
        "subscribed": subscribed,
        "subscribers": subscribers,
    }))
    .into_response()
}

pub async fn api_delete_video(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(video_id): Path<i64>,
) -> Response {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT user_id, file_path FROM aextube_videos WHERE id = ?",
    )
    .bind(video_id)
    .fetch_optional(&state.db.pool)
    .await
    .unwrap_or(None);

    let Some((owner_id, file_path)) = row else {
        return Json(serde_json::json!({ "error": "no_encontrado" })).into_response();
    };

    let can_moderate = state.db.user_has_perm(&user, "admin.access").await;
    if owner_id != user.id && !can_moderate {
        return Json(serde_json::json!({ "error": "no_autorizado" })).into_response();
    }

    let _ = sqlx::query("DELETE FROM aextube_videos WHERE id = ?")
        .bind(video_id)
        .execute(&state.db.pool)
        .await;

    if let Some(fname) = file_path.strip_prefix("/aextube/media/") {
        if !fname.contains("..") && !fname.contains('/') {
            let _ = std::fs::remove_file(format!("{}/{}", MEDIA_DIR, fname));
        }
    }

    Json(serde_json::json!({ "ok": true })).into_response()
}
