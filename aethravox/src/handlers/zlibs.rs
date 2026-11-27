#![allow(dead_code)]

use std::collections::HashSet;

use aethravox_core::{
    ZLibAccountPayload, ZLibCreateTokenRequest, ZLibCreateTokenResponse, ZLibFileInput,
    ZLibPackageDetailDto, ZLibPackageDto, ZLibPublishRequest, ZLibPublishResponse, ZLibTokenDto,
    ZLibVersionDto,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    auth_mw::AuthUser,
    config::Config,
    db::zlibs::{ZenCliTokenRow, ZenPackageSummaryRow, ZenPackageVersionRow},
    models::User,
    AppState,
};

// ============================================================
// Helpers compartidos (validación, URLs, DTOs)
// ============================================================

pub fn effective_public_url(cfg: &Config, headers: &HeaderMap) -> String {
    let configured = cfg.public_url.trim().trim_end_matches('/');
    let is_local_cfg = configured.is_empty()
        || configured.contains("localhost")
        || configured.contains("127.0.0.1");

    if is_local_cfg {
        let host = headers
            .get("x-forwarded-host")
            .or_else(|| headers.get(header::HOST))
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if let Some(h) = host {
            let proto = headers
                .get("x-forwarded-proto")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| *s == "http" || *s == "https")
                .unwrap_or_else(|| {
                    if h.starts_with("127.0.0.1") || h.starts_with("localhost") {
                        "http"
                    } else {
                        "https"
                    }
                });
            return format!("{proto}://{h}");
        }
    }

    if configured.is_empty() {
        "http://127.0.0.1:8080".to_string()
    } else {
        configured.to_string()
    }
}

pub fn is_valid_zen_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn is_reserved_zen_name(name_lower: &str) -> bool {
    name_lower.ends_with("zl")
        || matches!(
            name_lower,
            "3dz"
                | "fz"
                | "hz"
                | "llvmz"
                | "rz"
                | "sz"
                | "vgz"
                | "vmz"
                | "zen"
                | "std"
                | "core"
        )
}

pub fn normalize_zen_version(raw: &str) -> Option<String> {
    let v = raw.trim().trim_start_matches(['v', 'V']);
    if v.is_empty()
        || v.len() > 64
        || v.starts_with('.')
        || v.ends_with('.')
        || v.ends_with(".lock")
        || v.contains("..")
        || !v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
    {
        return None;
    }
    Some(v.to_string())
}

fn sanitize_zen_rel_path(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().replace('\\', "/");
    let stripped = normalized.trim_start_matches("./");
    if stripped.is_empty() || stripped.starts_with('/') || stripped.ends_with('/') {
        return Err(format!("Ruta de archivo inválida: '{}'.", raw));
    }
    let parts: Vec<&str> = stripped.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == ".." || p.chars().any(|c| c.is_control()))
    {
        return Err(format!("Ruta de archivo insegura: '{}'.", raw));
    }
    if parts[0] == ".git" || parts[0] == "zentarget" || parts[0] == "target" {
        return Err(format!("Directorio excluido en publicación: '{}'.", raw));
    }
    Ok(parts.join("/"))
}

fn extract_description_from_files(manifest: &str, readme: &str, pkg_name: &str) -> String {
    for line in manifest.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(rest) = lower
            .strip_prefix("description")
            .or_else(|| lower.strip_prefix("desc"))
        {
            if rest.trim_start().starts_with('=') || rest.trim_start().starts_with(':') {
                if let Some((_, val)) = trimmed.split_once(['=', ':']) {
                    let clean = val.trim().trim_matches('"').trim_matches('\'').trim();
                    if !clean.is_empty() {
                        return clean.chars().take(240).collect();
                    }
                }
            }
        }
    }
    for line in readme.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("```")
            || trimmed.starts_with("![")
        {
            continue;
        }
        return trimmed.chars().take(240).collect();
    }
    format!("Librería Zen {}", pkg_name)
}

pub fn token_row_to_dto(row: ZenCliTokenRow) -> ZLibTokenDto {
    ZLibTokenDto {
        id: row.id,
        label: row.label,
        token_prefix: row.token_prefix,
        created_at: row.created_at,
        last_used_at: row.last_used_at,
    }
}

pub fn version_row_to_dto(pkg_name: &str, v: ZenPackageVersionRow) -> ZLibVersionDto {
    ZLibVersionDto {
        dep_line: format!("dep {} \"{}\"", pkg_name, v.version),
        version: v.version,
        git_ref: v.git_ref,
        files_count: v.files_count,
        created_at: v.created_at,
    }
}

pub fn summary_row_to_dto(
    pkg: ZenPackageSummaryRow,
    versions: Vec<ZenPackageVersionRow>,
) -> ZLibPackageDto {
    let dep_line = format!("dep {} \"{}\"", pkg.name, pkg.latest_version);
    let version_dtos = versions
        .into_iter()
        .map(|v| version_row_to_dto(&pkg.name, v))
        .collect();
    ZLibPackageDto {
        id: pkg.id,
        name: pkg.name,
        owner_username: pkg.owner_username,
        description: pkg.description,
        git_url: pkg.git_url,
        latest_version: pkg.latest_version,
        dep_line,
        downloads_count: pkg.downloads_count,
        updated_at: pkg.updated_at,
        versions: version_dtos,
    }
}

pub async fn build_zlib_account_payload(
    state: &AppState,
    user_id: i64,
    public_url: &str,
) -> ZLibAccountPayload {
    let tokens = state
        .db
        .list_user_zen_cli_tokens(user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(token_row_to_dto)
        .collect();
    let my_packages = state
        .db
        .list_user_zen_packages(user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(p, vers)| summary_row_to_dto(p, vers))
        .collect();
    ZLibAccountPayload {
        server_url: public_url.to_string(),
        tokens,
        my_packages,
    }
}

// ============================================================
// Cliente GitHub REST API (creación de repo + snapshot por tag)
// ============================================================

#[derive(Deserialize)]
struct GhErrorBody {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct GhUserResp {
    login: String,
}

#[derive(Deserialize)]
struct GhRepoResp {
    #[serde(default = "default_branch_name")]
    default_branch: String,
}

fn default_branch_name() -> String {
    "main".to_string()
}

#[derive(Deserialize)]
struct GhRefResp {
    object: GhRefObject,
}

#[derive(Deserialize)]
struct GhRefObject {
    sha: String,
}

#[derive(Deserialize)]
struct GhShaResp {
    sha: String,
}

#[derive(Serialize)]
struct GhTreeEntry<'a> {
    path: &'a str,
    mode: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    sha: &'a str,
}

fn gh_headers(token: &str) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    map.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("Aethravox-ZLibs/1.0"),
    );
    map.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
    );
    map.insert(
        reqwest::header::HeaderName::from_static("x-github-api-version"),
        reqwest::header::HeaderValue::from_static("2022-11-28"),
    );
    if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token.trim())) {
        map.insert(reqwest::header::AUTHORIZATION, val);
    }
    map
}

async fn gh_err_msg(resp: reqwest::Response) -> String {
    let status = resp.status();
    let txt = resp.text().await.unwrap_or_default();
    if let Ok(parsed) = serde_json::from_str::<GhErrorBody>(&txt) {
        if !parsed.message.is_empty() {
            return format!("GitHub API ({status}): {}", parsed.message);
        }
    }
    format!("GitHub API ({status}): {}", txt.chars().take(240).collect::<String>())
}

async fn push_package_to_github(
    cfg: &Config,
    pkg_name: &str,
    version: &str,
    description: &str,
    files: &[ZLibFileInput],
) -> Result<String, (StatusCode, String)> {
    let gh_token = cfg.github_token.trim();
    if gh_token.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "GITHUB_TOKEN no está configurado en el servidor Aethravox.".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .default_headers(gh_headers(gh_token))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("No se pudo inicializar el cliente HTTP de GitHub: {e}"),
            )
        })?;

    let (owner, is_org) = if !cfg.github_org.trim().is_empty() {
        (cfg.github_org.trim().to_string(), true)
    } else if !cfg.github_owner.trim().is_empty() {
        (cfg.github_owner.trim().to_string(), false)
    } else {
        let user_resp = client
            .get("https://api.github.com/user")
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Error conectando con GitHub (/user): {e}"),
                )
            })?;
        if !user_resp.status().is_success() {
            let msg = gh_err_msg(user_resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
        let u: GhUserResp = user_resp.json().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Respuesta inválida de GitHub (/user): {e}"),
            )
        })?;
        (u.login, false)
    };

    // 1) Comprobar si el repositorio existe; si no, crearlo público.
    let repo_get_url = format!("https://api.github.com/repos/{owner}/{pkg_name}");
    let repo_check = client.get(&repo_get_url).send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Error consultando repositorio en GitHub: {e}"),
        )
    })?;

    let mut created_now = false;
    let default_branch = if repo_check.status() == reqwest::StatusCode::NOT_FOUND {
        let create_url = if is_org {
            format!("https://api.github.com/orgs/{owner}/repos")
        } else {
            "https://api.github.com/user/repos".to_string()
        };
        let create_resp = client
            .post(&create_url)
            .json(&json!({
                "name": pkg_name,
                "description": description,
                "private": false,
                "auto_init": true
            }))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Error creando repositorio en GitHub: {e}"),
                )
            })?;
        if !create_resp.status().is_success() {
            let msg = gh_err_msg(create_resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
        created_now = true;
        let created: GhRepoResp = create_resp.json().await.unwrap_or(GhRepoResp {
            default_branch: "main".to_string(),
        });
        created.default_branch
    } else if repo_check.status().is_success() {
        let info: GhRepoResp = repo_check.json().await.unwrap_or(GhRepoResp {
            default_branch: "main".to_string(),
        });
        info.default_branch
    } else {
        let msg = gh_err_msg(repo_check).await;
        return Err((StatusCode::BAD_GATEWAY, msg));
    };

    // 2) Obtener el SHA HEAD de la rama por defecto (reintentando si se acaba de auto-inicializar).
    let head_ref_url = format!(
        "https://api.github.com/repos/{owner}/{pkg_name}/git/ref/heads/{default_branch}"
    );
    let mut parent_sha: Option<String> = None;
    let mut repo_is_empty = false;

    for attempt in 0..5 {
        let ref_resp = client.get(&head_ref_url).send().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error leyendo rama {default_branch} en GitHub: {e}"),
            )
        })?;
        if ref_resp.status().is_success() {
            let parsed: GhRefResp = ref_resp.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Respuesta inválida de git ref en GitHub: {e}"),
                )
            })?;
            parent_sha = Some(parsed.object.sha);
            break;
        } else if ref_resp.status() == reqwest::StatusCode::CONFLICT {
            repo_is_empty = true;
            break;
        } else if created_now && ref_resp.status() == reqwest::StatusCode::NOT_FOUND && attempt < 4
        {
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
            continue;
        } else {
            let msg = gh_err_msg(ref_resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
    }

    // Si el repo existía pero estaba 100% vacío, inicializar con el primer archivo vía Contents API.
    if repo_is_empty || parent_sha.is_none() {
        let first = &files[0];
        let content_url = format!(
            "https://api.github.com/repos/{owner}/{pkg_name}/contents/{}",
            first.path
        );
        let b64 = base64::engine::general_purpose::STANDARD.encode(first.content.as_bytes());
        let put_resp = client
            .put(&content_url)
            .json(&json!({
                "message": format!("Initialize {pkg_name} {version}"),
                "content": b64,
                "branch": default_branch
            }))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Error inicializando repositorio vacío en GitHub: {e}"),
                )
            })?;
        if !put_resp.status().is_success() {
            let msg = gh_err_msg(put_resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
        let ref_resp = client.get(&head_ref_url).send().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error leyendo HEAD tras inicializar GitHub: {e}"),
            )
        })?;
        if !ref_resp.status().is_success() {
            let msg = gh_err_msg(ref_resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
        let parsed: GhRefResp = ref_resp.json().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Respuesta inválida de HEAD en GitHub: {e}"),
            )
        })?;
        parent_sha = Some(parsed.object.sha);
    }

    let parent_sha = parent_sha.ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "No se pudo resolver el commit padre en GitHub.".to_string(),
        )
    })?;

    // 3) Crear blobs para todos los files[]
    let blobs_url = format!("https://api.github.com/repos/{owner}/{pkg_name}/git/blobs");
    let mut blob_shas: Vec<String> = Vec::with_capacity(files.len());
    for f in files {
        let resp = client
            .post(&blobs_url)
            .json(&json!({
                "content": f.content,
                "encoding": "utf-8"
            }))
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Error subiendo archivo '{}' a GitHub: {e}", f.path),
                )
            })?;
        if !resp.status().is_success() {
            let msg = gh_err_msg(resp).await;
            return Err((StatusCode::BAD_GATEWAY, msg));
        }
        let parsed: GhShaResp = resp.json().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Respuesta inválida al crear blob en GitHub: {e}"),
            )
        })?;
        blob_shas.push(parsed.sha);
    }

    // 4) Crear tree limpio (sin base_tree) con los archivos exactos de esta versión.
    let tree_entries: Vec<GhTreeEntry<'_>> = files
        .iter()
        .zip(blob_shas.iter())
        .map(|(f, sha)| GhTreeEntry {
            path: &f.path,
            mode: "100644",
            kind: "blob",
            sha,
        })
        .collect();

    let trees_url = format!("https://api.github.com/repos/{owner}/{pkg_name}/git/trees");
    let tree_resp = client
        .post(&trees_url)
        .json(&json!({ "tree": tree_entries }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error creando tree en GitHub: {e}"),
            )
        })?;
    if !tree_resp.status().is_success() {
        let msg = gh_err_msg(tree_resp).await;
        return Err((StatusCode::BAD_GATEWAY, msg));
    }
    let tree_sha = tree_resp
        .json::<GhShaResp>()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Respuesta inválida de tree en GitHub: {e}"),
            )
        })?
        .sha;

    // 5) Crear commit de release.
    let commits_url = format!("https://api.github.com/repos/{owner}/{pkg_name}/git/commits");
    let commit_resp = client
        .post(&commits_url)
        .json(&json!({
            "message": format!("Release {pkg_name} {version}"),
            "tree": tree_sha,
            "parents": [parent_sha]
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error creando commit en GitHub: {e}"),
            )
        })?;
    if !commit_resp.status().is_success() {
        let msg = gh_err_msg(commit_resp).await;
        return Err((StatusCode::BAD_GATEWAY, msg));
    }
    let commit_sha = commit_resp
        .json::<GhShaResp>()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Respuesta inválida de commit en GitHub: {e}"),
            )
        })?
        .sha;

    // 6) Actualizar rama por defecto al último release para que se vea el código al entrar a GitHub.
    let patch_resp = client
        .patch(&head_ref_url)
        .json(&json!({
            "sha": commit_sha,
            "force": true
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error actualizando rama {default_branch} en GitHub: {e}"),
            )
        })?;
    if !patch_resp.status().is_success() {
        let msg = gh_err_msg(patch_resp).await;
        return Err((StatusCode::BAD_GATEWAY, msg));
    }

    // 7) Crear tag inmutable `refs/tags/<version>` para `git clone --branch <version>`.
    let create_tag_url = format!("https://api.github.com/repos/{owner}/{pkg_name}/git/refs");
    let tag_resp = client
        .post(&create_tag_url)
        .json(&json!({
            "ref": format!("refs/tags/{version}"),
            "sha": commit_sha
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Error creando tag '{version}' en GitHub: {e}"),
            )
        })?;
    if tag_resp.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
        return Err((
            StatusCode::CONFLICT,
            format!("El tag '{version}' ya existe en el repositorio de GitHub."),
        ));
    }
    if !tag_resp.status().is_success() {
        let msg = gh_err_msg(tag_resp).await;
        return Err((StatusCode::BAD_GATEWAY, msg));
    }

    Ok(format!("https://github.com/{owner}/{pkg_name}.git"))
}

// ============================================================
// Lógica central de publicación (usada por CLI, Web y Nativa)
// ============================================================

pub async fn publish_zen_package_core(
    state: &AppState,
    actor: &User,
    req: ZLibPublishRequest,
) -> Result<ZLibPublishResponse, (StatusCode, String)> {
    let raw_name = req.name.trim();
    if !is_valid_zen_name(raw_name) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Nombre inválido: usa solo [a-zA-Z0-9_-] (máximo 64 caracteres).".to_string(),
        ));
    }
    let pkg_name = raw_name.to_ascii_lowercase();
    if is_reserved_zen_name(&pkg_name) {
        return Err((
            StatusCode::FORBIDDEN,
            format!(
                "El nombre '{pkg_name}' está reservado para módulos oficiales de Zen."
            ),
        ));
    }

    let Some(version) = normalize_zen_version(&req.version) else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Versión inválida (ej. usa 1.0.0 o 1.1.0).".to_string(),
        ));
    };

    if req.files.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Debes enviar al menos un archivo para publicar la librería.".to_string(),
        ));
    }
    if req.files.len() > 400 {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Demasiados archivos en el paquete (máximo 400).".to_string(),
        ));
    }

    let mut clean_files: Vec<ZLibFileInput> = Vec::with_capacity(req.files.len());
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut total_bytes: usize = 0;

    for f in req.files {
        let path = sanitize_zen_rel_path(&f.path).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        if !seen_paths.insert(path.clone()) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Archivo duplicado en la petición: '{path}'."),
            ));
        }
        total_bytes = total_bytes.saturating_add(f.content.len());
        if total_bytes > 20 * 1024 * 1024 {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "El tamaño total de la librería supera el límite de 20 MB.".to_string(),
            ));
        }
        clean_files.push(ZLibFileInput {
            path,
            content: f.content,
        });
    }

    // Dueño único + versión inmutable
    if let Ok(Some(existing)) = state.db.get_zen_package_by_name(&pkg_name).await {
        if existing.owner_id != actor.id {
            return Err((
                StatusCode::FORBIDDEN,
                format!("El paquete '{pkg_name}' ya pertenece a otro usuario."),
            ));
        }
        if let Ok(Some(_)) = state
            .db
            .get_zen_package_version(existing.id, &version)
            .await
        {
            return Err((
                StatusCode::CONFLICT,
                format!(
                    "La versión '{version}' de '{pkg_name}' ya existe y es inmutable."
                ),
            ));
        }
    }

    let readme = clean_files
        .iter()
        .find(|f| f.path.eq_ignore_ascii_case("README.md"))
        .map(|f| f.content.clone())
        .unwrap_or_default();

    let manifest = clean_files
        .iter()
        .find(|f| {
            f.path.eq_ignore_ascii_case("Make.mz")
                || f.path.eq_ignore_ascii_case("Zen.mz")
                || f.path.eq_ignore_ascii_case("Manifest.zm")
        })
        .map(|f| f.content.clone())
        .unwrap_or_default();

    let description = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(240).collect::<String>())
        .unwrap_or_else(|| extract_description_from_files(&manifest, &readme, &pkg_name));

    let git_url =
        push_package_to_github(&state.cfg, &pkg_name, &version, &description, &clean_files)
            .await?;

    let files_count = clean_files.len() as i64;
    let git_ref = version.clone();

    state
        .db
        .record_zen_package_publish(
            &pkg_name,
            actor.id,
            &description,
            &git_url,
            &readme,
            &version,
            &git_ref,
            &manifest,
            files_count,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error guardando el paquete en base de datos: {e}"),
            )
        })?;

    let _ = state
        .db
        .audit(
            Some(actor.id),
            "zlib_publish",
            Some(&format!("{pkg_name}@{version} -> {git_url}")),
            None,
        )
        .await;

    Ok(ZLibPublishResponse { git_url, git_ref })
}

// ============================================================
// Endpoints exactos del compilador Zen CLI (/api/zen/packages/*)
// ============================================================

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub async fn cli_publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let Some(token) = extract_bearer(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            "Token CLI requerido. Ejecuta primero: zen login --token <TU_TOKEN> --url <URL>\n",
        )
            .into_response();
    };

    let user = match state.db.authenticate_zen_cli_token(&token).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                "Token CLI inválido o revocado.\n",
            )
                .into_response();
        }
    };

    let req: ZLibPublishRequest = match serde_json::from_slice(&body) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("JSON inválido al publicar paquete: {e}\n"),
            )
                .into_response();
        }
    };

    match publish_zen_package_core(&state, &user, req).await {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, msg)) => (status, format!("{msg}\n")).into_response(),
    }
}

pub async fn cli_resolve_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> Response {
    let pkg_name = name.trim().to_ascii_lowercase();
    if pkg_name.is_empty() {
        return (StatusCode::BAD_REQUEST, "Nombre de paquete vacío.\n").into_response();
    }
    let clean_ver = version.trim().trim_start_matches(['v', 'V']);
    if clean_ver.is_empty() {
        return (StatusCode::BAD_REQUEST, "Versión vacía.\n").into_response();
    }

    let pkg = match state.db.get_zen_package_by_name(&pkg_name).await {
        Ok(Some(p)) => p,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                format!("Paquete '{pkg_name}' no encontrado en ZLibs.\n"),
            )
                .into_response();
        }
    };

    let target_ver = if clean_ver.eq_ignore_ascii_case("latest") {
        pkg.latest_version.as_str()
    } else {
        clean_ver
    };

    let ver_row = match state
        .db
        .get_zen_package_version(pkg.id, target_ver)
        .await
    {
        Ok(Some(v)) => v,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                format!("Versión '{target_ver}' de '{pkg_name}' no encontrada.\n"),
            )
                .into_response();
        }
    };

    let _ = state.db.increment_zen_package_downloads(pkg.id).await;

    Json(ZLibPublishResponse {
        git_url: pkg.git_url,
        git_ref: ver_row.git_ref,
    })
    .into_response()
}

#[derive(Deserialize)]
pub struct ZLibSearchQuery {
    #[serde(default)]
    pub q: String,
}

pub async fn cli_search(
    State(state): State<AppState>,
    Query(query): Query<ZLibSearchQuery>,
) -> Response {
    let pkgs = state
        .db
        .search_zen_packages(&query.q, 40)
        .await
        .unwrap_or_default();

    if pkgs.is_empty() {
        let msg = if query.q.trim().is_empty() {
            "No hay librerías publicadas en ZLibs todavía.\n".to_string()
        } else {
            format!("No se encontraron librerías Zen para '{}'.\n", query.q.trim())
        };
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            msg,
        )
            .into_response();
    }

    let mut out = String::new();
    for p in pkgs {
        out.push_str(&format!(
            "{} (v{}) — {}\n  dep: dep {} \"{}\"\n  git: {}\n  autor: @{} · {} descargas\n\n",
            p.name,
            p.latest_version,
            p.description,
            p.name,
            p.latest_version,
            p.git_url,
            p.owner_username,
            p.downloads_count
        ));
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        out,
    )
        .into_response()
}

// ============================================================
// Askama Web UI (/zlibs, /zlibs/:name + endpoints cookie)
// ============================================================

#[derive(Template)]
#[template(path = "herramientas/zlibs/zlibs.html")]
pub struct ZLibsTemplate {
    pub username: String,
    pub is_guest: bool,
    pub public_url: String,
    pub q: String,
    pub packages: Vec<ZLibPackageDto>,
    pub my_packages: Vec<ZLibPackageDto>,
    pub my_tokens: Vec<ZLibTokenDto>,
}

#[derive(Template)]
#[template(path = "herramientas/zlibs/detail.html")]
pub struct ZLibDetailTemplate {
    pub username: String,
    pub is_guest: bool,
    pub pkg: ZLibPackageDto,
    pub readme: String,
    pub manifest: String,
}

pub async fn zlibs_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    Query(query): Query<ZLibSearchQuery>,
) -> Response {
    let public_url = effective_public_url(&state.cfg, &headers);
    let rows = state
        .db
        .search_zen_packages(&query.q, 60)
        .await
        .unwrap_or_default();

    let mut packages = Vec::with_capacity(rows.len());
    for row in rows {
        let vers = state
            .db
            .list_zen_package_versions(row.id)
            .await
            .unwrap_or_default();
        packages.push(summary_row_to_dto(row, vers));
    }

    let (my_packages, my_tokens) = if user.id > 0 {
        let acc = build_zlib_account_payload(&state, user.id, &public_url).await;
        (acc.my_packages, acc.tokens)
    } else {
        (Vec::new(), Vec::new())
    };

    ZLibsTemplate {
        username: user.username.clone(),
        is_guest: user.id == 0,
        public_url,
        q: query.q,
        packages,
        my_packages,
        my_tokens,
    }
    .into_response()
}

pub async fn zlibs_detail_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(name): Path<String>,
) -> Response {
    let pkg_name = name.trim().to_ascii_lowercase();
    let detail = match state.db.get_zen_package_detail(&pkg_name).await {
        Ok(Some(d)) => d,
        _ => return Redirect::to("/zlibs").into_response(),
    };
    let manifest = detail
        .versions
        .first()
        .map(|v| v.manifest.clone())
        .unwrap_or_default();
    let pkg_dto = summary_row_to_dto(detail.package, detail.versions);

    ZLibDetailTemplate {
        username: user.username.clone(),
        is_guest: user.id == 0,
        pkg: pkg_dto,
        readme: detail.readme,
        manifest,
    }
    .into_response()
}

pub async fn web_create_token(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if user.id == 0 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Debes iniciar sesión." })),
        )
            .into_response();
    }
    let label = serde_json::from_slice::<ZLibCreateTokenRequest>(&body)
        .map(|b| b.label)
        .unwrap_or_else(|_| "Zen CLI".to_string());
    let public_url = effective_public_url(&state.cfg, &headers);
    match state.db.create_zen_cli_token(user.id, &label).await {
        Ok((row, raw_token)) => {
            let cli_command = format!("zen login --token {raw_token} --url {public_url}");
            let _ = state
                .db
                .audit(Some(user.id), "zlib_token_create", Some(&row.token_prefix), None)
                .await;
            Json(ZLibCreateTokenResponse {
                token: raw_token,
                cli_command,
                server_url: public_url,
                item: token_row_to_dto(row),
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("No se pudo generar el token: {e}") })),
        )
            .into_response(),
    }
}

pub async fn web_revoke_token(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if user.id == 0 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Debes iniciar sesión." })),
        )
            .into_response();
    }
    match state.db.revoke_zen_cli_token(user.id, id).await {
        Ok(true) => {
            let _ = state
                .db
                .audit(Some(user.id), "zlib_token_revoke", Some(&format!("id={id}")), None)
                .await;
            Json(json!({ "ok": true })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Token no encontrado." })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Error revocando token: {e}") })),
        )
            .into_response(),
    }
}

pub async fn web_publish_package(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<ZLibPublishRequest>,
) -> Response {
    if user.id == 0 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Debes iniciar sesión." })),
        )
            .into_response();
    }
    match publish_zen_package_core(&state, &user, req).await {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err((status, msg)) => (status, Json(json!({ "error": msg }))).into_response(),
    }
}
