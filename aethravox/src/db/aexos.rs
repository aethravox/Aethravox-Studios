use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::FromRow;

use crate::models::User;

use super::{random_hex, verify_password, Db};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AexosAccountRow {
    pub user_id: i64,
    pub owner_user_id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub wallpaper: String,
    pub show_desktop_icons: i64,
    pub pinned_apps_json: String,
    pub files_json: String,
    pub notes_text: String,
}

impl Db {
    /// Asegura que cualquier usuario tenga su propia fila de preferencias de escritorio en `aexos_accounts`.
    pub async fn ensure_aexos_prefs(&self, user: &User) -> Result<()> {
        let default_files = serde_json::json!([
            {
                "name": "bienvenida.txt",
                "content": format!(
                    "Bienvenido a aexOS, @{}.\n\nTodo lo que abras aquí se ejecuta con tu propia cuenta de Aethravox Studios.",
                    user.username
                ),
                "created": "Hoy"
            }
        ])
        .to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO aexos_accounts \
             (user_id, owner_user_id, wallpaper, show_desktop_icons, pinned_apps_json, files_json, notes_text) \
             VALUES (?, ?, 'nebula', 1, '[\"files\",\"terminal\",\"aris\",\"chat\",\"aextube\"]', ?, '')",
        )
        .bind(user.id)
        .bind(user.id)
        .bind(&default_files)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ensure_primary_aexos_prefs(&self, owner: &User) -> Result<()> {
        self.ensure_aexos_prefs(owner).await
    }

    /// Lista la cuenta principal + todas las cuentas reales de Aethravox Studios vinculadas por `owner` en aexOS.
    /// Nota: si alguna cuenta fue vinculada antes, solo se guarda la referencia (no la contraseña).
    pub async fn list_aexos_accounts(&self, owner: &User) -> Result<Vec<AexosAccountRow>> {
        self.ensure_aexos_prefs(owner).await?;

        // Asegurar fila en `aexos_accounts` para cada usuario vinculado si aún no tenía escritorio creado.
        let linked_ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT linked_user_id FROM aexos_links WHERE owner_user_id = ?",
        )
        .bind(owner.id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for (uid,) in linked_ids {
            if let Ok(Some(u)) = self.find_user_by_id(uid).await {
                let _ = self.ensure_aexos_prefs(&u).await;
            }
        }

        let rows = sqlx::query_as::<_, AexosAccountRow>(
            "SELECT a.user_id, ? AS owner_user_id, u.username, u.display_name, u.avatar_url, u.role, \
                    a.wallpaper, a.show_desktop_icons, a.pinned_apps_json, a.files_json, a.notes_text \
             FROM aexos_accounts a \
             JOIN users u ON u.id = a.user_id \
             WHERE a.user_id = ? \
                OR EXISTS ( \
                    SELECT 1 FROM aexos_links l \
                    WHERE l.owner_user_id = ? AND l.linked_user_id = a.user_id \
                ) \
             ORDER BY CASE WHEN a.user_id = ? THEN 0 ELSE 1 END, u.id ASC",
        )
        .bind(owner.id)
        .bind(owner.id)
        .bind(owner.id)
        .bind(owner.id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Compatibilidad con login normal: solo bloquea si existiera una cuenta sintética legada (`!aexos_only!`).
    pub async fn is_aexos_only_user(&self, user_id: i64) -> bool {
        sqlx::query_as::<_, (i64,)>(
            "SELECT 1 FROM users WHERE id = ? AND password_hash = '!aexos_only!' LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .is_some()
    }

    /// Verifica credenciales de una cuenta real de Aethravox Studios y la vincula (guardando solo el ID/nombre, jamás la contraseña).
    pub async fn link_existing_aexos_account(
        &self,
        owner: &User,
        raw_identifier: &str,
        password: &str,
    ) -> Result<AexosAccountRow> {
        let identifier = raw_identifier.trim().trim_start_matches('@').to_ascii_lowercase();
        if identifier.is_empty() {
            return Err(anyhow!("Ingresa el usuario o correo de la cuenta de Aethravox Studios."));
        }
        if password.is_empty() {
            return Err(anyhow!("Ingresa la contraseña de esa cuenta para verificarla."));
        }

        let target = if identifier.contains('@') {
            self.find_user_by_email(&identifier).await?
        } else {
            self.find_user_by_username(&identifier).await?
        }
        .ok_or_else(|| anyhow!("No existe ninguna cuenta en Aethravox Studios con ese usuario."))?;

        if target.id == owner.id {
            return Err(anyhow!("Ya estás usando @{} como cuenta principal.", owner.username));
        }

        if !verify_password(password, &target.password_hash) {
            return Err(anyhow!("Contraseña incorrecta para @{}.", target.username));
        }

        if self.is_user_banned(target.id).await.ok().flatten().is_some() {
            return Err(anyhow!("La cuenta @{} está suspendida permanentemente.", target.username));
        }
        if self.is_user_suspended(target.id).await.ok().flatten().is_some() {
            return Err(anyhow!("La cuenta @{} está suspendida temporalmente.", target.username));
        }

        let already_linked: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?",
        )
        .bind(owner.id)
        .bind(target.id)
        .fetch_optional(&self.pool)
        .await?;
        if already_linked.is_some() {
            return Err(anyhow!("La cuenta @{} ya está guardada en tu aexOS.", target.username));
        }

        sqlx::query(
            "INSERT INTO aexos_links (owner_user_id, linked_user_id) VALUES (?, ?)",
        )
        .bind(owner.id)
        .bind(target.id)
        .execute(&self.pool)
        .await?;

        self.ensure_aexos_prefs(&target).await?;

        let row = sqlx::query_as::<_, AexosAccountRow>(
            "SELECT a.user_id, ? AS owner_user_id, u.username, u.display_name, u.avatar_url, u.role, \
                    a.wallpaper, a.show_desktop_icons, a.pinned_apps_json, a.files_json, a.notes_text \
             FROM aexos_accounts a \
             JOIN users u ON u.id = a.user_id \
             WHERE a.user_id = ?",
        )
        .bind(owner.id)
        .bind(target.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Desvincula una cuenta secundaria de la lista de aexOS del propietario (NUNCA borra la cuenta real de `users`).
    pub async fn delete_aexos_account(&self, owner_user_id: i64, target_user_id: i64) -> Result<()> {
        if owner_user_id == target_user_id {
            return Err(anyhow!("No puedes quitar tu cuenta principal."));
        }
        let linked: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?",
        )
        .bind(owner_user_id)
        .bind(target_user_id)
        .fetch_optional(&self.pool)
        .await?;
        if linked.is_none() {
            return Err(anyhow!("Usuario vinculado no encontrado en aexOS."));
        }
        sqlx::query("DELETE FROM aexos_sessions WHERE owner_user_id = ? AND profile_user_id = ?")
            .bind(owner_user_id)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?")
            .bind(owner_user_id)
            .bind(target_user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_aexos_prefs(
        &self,
        owner_user_id: i64,
        profile_user_id: i64,
        wallpaper: &str,
        show_desktop_icons: bool,
        pinned_apps_json: &str,
        files_json: &str,
        notes_text: &str,
    ) -> Result<()> {
        let allowed = profile_user_id == owner_user_id
            || sqlx::query_as::<_, (i64,)>(
                "SELECT 1 FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?",
            )
            .bind(owner_user_id)
            .bind(profile_user_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

        if !allowed {
            return Err(anyhow!("No tienes permiso para guardar preferencias de esta cuenta."));
        }

        sqlx::query(
            "UPDATE aexos_accounts \
             SET wallpaper = ?, show_desktop_icons = ?, pinned_apps_json = ?, files_json = ?, notes_text = ? \
             WHERE user_id = ?",
        )
        .bind(wallpaper)
        .bind(if show_desktop_icons { 1i64 } else { 0i64 })
        .bind(pinned_apps_json)
        .bind(files_json)
        .bind(notes_text)
        .bind(profile_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Abre una sesión temporal dentro de aexOS para una cuenta vinculada tras verificar su contraseña real.
    pub async fn verify_and_create_aexos_session(
        &self,
        owner_user_id: i64,
        profile_user_id: i64,
        password: Option<&str>,
    ) -> Result<String> {
        let linked: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?",
        )
        .bind(owner_user_id)
        .bind(profile_user_id)
        .fetch_optional(&self.pool)
        .await?;
        if linked.is_none() {
            return Err(anyhow!("Esta cuenta no está vinculada en tu aexOS."));
        }

        let target = self
            .find_user_by_id(profile_user_id)
            .await?
            .ok_or_else(|| anyhow!("Cuenta de usuario no encontrada."))?;

        let pw = password.unwrap_or("");
        if pw.is_empty() || !verify_password(pw, &target.password_hash) {
            return Err(anyhow!("Contraseña incorrecta para @{}.", target.username));
        }
        if self.is_user_banned(target.id).await.ok().flatten().is_some() {
            return Err(anyhow!("La cuenta @{} está suspendida.", target.username));
        }
        if self.is_user_suspended(target.id).await.ok().flatten().is_some() {
            return Err(anyhow!("La cuenta @{} está suspendida temporalmente.", target.username));
        }

        self.create_aexos_session(owner_user_id, profile_user_id).await
    }

    pub async fn create_aexos_session(
        &self,
        owner_user_id: i64,
        profile_user_id: i64,
    ) -> Result<String> {
        let allowed: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM aexos_links WHERE owner_user_id = ? AND linked_user_id = ?",
        )
        .bind(owner_user_id)
        .bind(profile_user_id)
        .fetch_optional(&self.pool)
        .await?;
        if allowed.is_none() && profile_user_id != owner_user_id {
            return Err(anyhow!("No tienes permiso para abrir este perfil en aexOS."));
        }
        sqlx::query("DELETE FROM aexos_sessions WHERE owner_user_id = ?")
            .bind(owner_user_id)
            .execute(&self.pool)
            .await?;

        let token = random_hex(24);
        let expires = Utc::now() + Duration::days(30);
        sqlx::query(
            "INSERT INTO aexos_sessions (token, owner_user_id, profile_user_id, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&token)
        .bind(owner_user_id)
        .bind(profile_user_id)
        .bind(expires)
        .execute(&self.pool)
        .await?;
        Ok(token)
    }

    pub async fn get_aexos_session_user(&self, owner_user_id: i64, token: &str) -> Option<User> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT profile_user_id FROM aexos_sessions \
             WHERE token = ? AND owner_user_id = ? AND expires_at > ?",
        )
        .bind(token)
        .bind(owner_user_id)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();
        let profile_id = row?.0;
        self.find_user_by_id(profile_id).await.ok().flatten()
    }

    pub async fn delete_aexos_session_by_token(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM aexos_sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_aexos_sessions_for_owner(&self, owner_user_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM aexos_sessions WHERE owner_user_id = ?")
            .bind(owner_user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
