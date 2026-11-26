#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

pub fn validate_username(s: &str) -> Result<(), String> {
    let len = s.chars().count();
    if !(3..=32).contains(&len) {
        return Err("El nombre de usuario debe tener entre 3 y 32 caracteres.".into());
    }
    for c in s.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
        if !ok {
            return Err("Solo se permiten letras, números, '_', '-' y '.'".into());
        }
    }
    Ok(())
}

pub fn validate_password(s: &str) -> Result<(), String> {
    if s.len() < 8 {
        return Err("La contraseña debe tener al menos 8 caracteres.".into());
    }
    if s.eq_ignore_ascii_case("password")
        || s == "12345678"
        || s == "cambiar_este_password_ya"
    {
        return Err("Esa contraseña es demasiado débil.".into());
    }
    Ok(())
}

pub const ALLOWED_INTERNAL_DOMAINS: &[&str] = &["aethra.sac", "shadow.sac", "cents.sac"];

pub fn validate_email(s: &str) -> Result<(), String> {
    let trimmed = s.trim();
    if trimmed.len() < 5 || trimmed.len() > 254 || trimmed.contains(' ') {
        return Err("Email inválido.".into());
    }
    if trimmed.matches('@').count() != 1 {
        return Err("Email inválido.".into());
    }
    let (local, domain) = trimmed.split_once('@').ok_or("Email inválido.")?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err("Email inválido.".into());
    }
    let domain_lower = domain.to_ascii_lowercase();
    if domain_lower == "gmail.com"
        || domain_lower == "googlemail.com"
        || domain_lower.starts_with("gmail.")
        || domain_lower.starts_with("googlemail.")
    {
        return Err("No se aceptan correos de Gmail por ahora. Usa una cuenta de AethraMail (@aethra.sac, @shadow.sac o @cents.sac).".into());
    }
    if !ALLOWED_INTERNAL_DOMAINS.contains(&domain_lower.as_str()) {
        return Err("Por ahora solo se aceptan correos internos de AethraMail (@aethra.sac, @shadow.sac o @cents.sac).".into());
    }
    Ok(())
}

pub fn validate_slug(s: &str) -> Result<(), String> {
    if s.len() < 2 || s.len() > 32 {
        return Err("El slug debe tener entre 2 y 32 caracteres.".into());
    }
    for c in s.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if !ok {
            return Err("Slug solo admite letras, números, '-' y '_'.".into());
        }
    }
    Ok(())
}

pub fn sanitize_text(s: &str, max: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() > max {
        trimmed.chars().take(max).collect()
    } else {
        trimmed.to_string()
    }
}

