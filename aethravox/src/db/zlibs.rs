#![allow(unused_imports)]

use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::FromRow;

use crate::models::User;
use super::{random_hex, Db};

pub fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(FromRow, Debug, Clone)]
pub struct ZenCliTokenRow {
    pub id: i64,
    pub user_id: i64,
    pub label: String,
    pub token_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(FromRow, Debug, Clone)]
pub struct ZenPackageRow {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub description: String,
    pub git_url: String,
    pub readme: String,
    pub latest_version: String,
    pub downloads_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct ZenPackageVersionRow {
    pub id: i64,
    pub package_id: i64,
    pub version: String,
    pub git_ref: String,
    pub manifest: String,
    pub files_count: i64,
    pub created_at: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct ZenPackageSummaryRow {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub owner_username: String,
    pub description: String,
    pub git_url: String,
    pub latest_version: String,
    pub downloads_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ZenPackageDetailRow {
    pub package: ZenPackageSummaryRow,
    pub readme: String,
    pub versions: Vec<ZenPackageVersionRow>,
}

impl Db {
    pub async fn create_zen_cli_token(
        &self,
        user_id: i64,
        label: &str,
    ) -> Result<(ZenCliTokenRow, String)> {
        let raw_token = format!("zen_{}", random_hex(24));
        let token_hash = sha256_hex(&raw_token);
        let prefix = format!("{}...", &raw_token[..12.min(raw_token.len())]);
        let clean_label = if label.trim().is_empty() {
            "Zen CLI"
        } else {
            label.trim()
        };
        let res = sqlx::query(
            "INSERT INTO zen_cli_tokens (user_id, label, token_hash, token_prefix)
             VALUES (?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(clean_label)
        .bind(&token_hash)
        .bind(&prefix)
        .execute(&self.pool)
        .await?;
        let id = res.last_insert_rowid();
        let row = sqlx::query_as::<_, ZenCliTokenRow>(
            "SELECT id, user_id, label, token_prefix, created_at, last_used_at
             FROM zen_cli_tokens WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok((row, raw_token))
    }

    pub async fn list_user_zen_cli_tokens(&self, user_id: i64) -> Result<Vec<ZenCliTokenRow>> {
        let rows = sqlx::query_as::<_, ZenCliTokenRow>(
            "SELECT id, user_id, label, token_prefix, created_at, last_used_at
             FROM zen_cli_tokens
             WHERE user_id = ?
             ORDER BY id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn revoke_zen_cli_token(&self, user_id: i64, token_id: i64) -> Result<bool> {
        let res = sqlx::query("DELETE FROM zen_cli_tokens WHERE id = ? AND user_id = ?")
            .bind(token_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn authenticate_zen_cli_token(&self, raw_token: &str) -> Result<Option<User>> {
        let trimmed = raw_token.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let token_hash = sha256_hex(trimmed);
        let found: Option<(i64, i64)> = sqlx::query_as(
            "SELECT id, user_id FROM zen_cli_tokens WHERE token_hash = ?",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;
        let Some((tok_id, user_id)) = found else {
            return Ok(None);
        };
        let _ = sqlx::query(
            "UPDATE zen_cli_tokens
             SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?",
        )
        .bind(tok_id)
        .execute(&self.pool)
        .await;
        match self.find_user_by_id(user_id).await? {
            Some(u) if u.status == "active" => Ok(Some(u)),
            _ => Ok(None),
        }
    }

    pub async fn get_zen_package_by_name(&self, name_lower: &str) -> Result<Option<ZenPackageRow>> {
        let row = sqlx::query_as::<_, ZenPackageRow>(
            "SELECT id, name, owner_id, description, git_url, readme, latest_version,
                    downloads_count, created_at, updated_at
             FROM zen_packages
             WHERE name = ?",
        )
        .bind(name_lower)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_zen_package_version(
        &self,
        package_id: i64,
        version: &str,
    ) -> Result<Option<ZenPackageVersionRow>> {
        let row = sqlx::query_as::<_, ZenPackageVersionRow>(
            "SELECT id, package_id, version, git_ref, manifest, files_count, created_at
             FROM zen_package_versions
             WHERE package_id = ? AND version = ?",
        )
        .bind(package_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_zen_package_versions(
        &self,
        package_id: i64,
    ) -> Result<Vec<ZenPackageVersionRow>> {
        let rows = sqlx::query_as::<_, ZenPackageVersionRow>(
            "SELECT id, package_id, version, git_ref, manifest, files_count, created_at
             FROM zen_package_versions
             WHERE package_id = ?
             ORDER BY id DESC",
        )
        .bind(package_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_zen_package_publish(
        &self,
        name_lower: &str,
        owner_id: i64,
        description: &str,
        git_url: &str,
        readme: &str,
        version: &str,
        git_ref: &str,
        manifest: &str,
        files_count: i64,
    ) -> Result<ZenPackageRow> {
        let mut tx = self.pool.begin().await?;
        let existing_id: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM zen_packages WHERE name = ?")
                .bind(name_lower)
                .fetch_optional(&mut *tx)
                .await?;
        let pkg_id = if let Some((id,)) = existing_id {
            sqlx::query(
                "UPDATE zen_packages
                 SET description = CASE WHEN ? != '' THEN ? ELSE description END,
                     git_url = ?,
                     readme = CASE WHEN ? != '' THEN ? ELSE readme END,
                     latest_version = ?,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?",
            )
            .bind(description)
            .bind(description)
            .bind(git_url)
            .bind(readme)
            .bind(readme)
            .bind(version)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            id
        } else {
            let res = sqlx::query(
                "INSERT INTO zen_packages
                    (name, owner_id, description, git_url, readme, latest_version)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(name_lower)
            .bind(owner_id)
            .bind(description)
            .bind(git_url)
            .bind(readme)
            .bind(version)
            .execute(&mut *tx)
            .await?;
            res.last_insert_rowid()
        };

        sqlx::query(
            "INSERT INTO zen_package_versions
                (package_id, version, git_ref, manifest, files_count)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(pkg_id)
        .bind(version)
        .bind(git_ref)
        .bind(manifest)
        .bind(files_count)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let pkg = sqlx::query_as::<_, ZenPackageRow>(
            "SELECT id, name, owner_id, description, git_url, readme, latest_version,
                    downloads_count, created_at, updated_at
             FROM zen_packages WHERE id = ?",
        )
        .bind(pkg_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(pkg)
    }

    pub async fn increment_zen_package_downloads(&self, package_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE zen_packages SET downloads_count = downloads_count + 1 WHERE id = ?",
        )
        .bind(package_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn search_zen_packages(
        &self,
        q: &str,
        limit: i64,
    ) -> Result<Vec<ZenPackageSummaryRow>> {
        let trimmed = q.trim().to_lowercase();
        let rows = if trimmed.is_empty() {
            sqlx::query_as::<_, ZenPackageSummaryRow>(
                "SELECT p.id, p.name, p.owner_id,
                        COALESCE(u.username, 'desconocido') AS owner_username,
                        p.description, p.git_url, p.latest_version, p.downloads_count, p.updated_at
                 FROM zen_packages p
                 LEFT JOIN users u ON u.id = p.owner_id
                 ORDER BY p.downloads_count DESC, p.updated_at DESC
                 LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            let pat = format!("%{trimmed}%");
            sqlx::query_as::<_, ZenPackageSummaryRow>(
                "SELECT p.id, p.name, p.owner_id,
                        COALESCE(u.username, 'desconocido') AS owner_username,
                        p.description, p.git_url, p.latest_version, p.downloads_count, p.updated_at
                 FROM zen_packages p
                 LEFT JOIN users u ON u.id = p.owner_id
                 WHERE LOWER(p.name) LIKE ?
                    OR LOWER(p.description) LIKE ?
                    OR LOWER(COALESCE(u.username, '')) LIKE ?
                 ORDER BY CASE WHEN LOWER(p.name) = ? THEN 0 WHEN LOWER(p.name) LIKE ? THEN 1 ELSE 2 END,
                          p.downloads_count DESC,
                          p.updated_at DESC
                 LIMIT ?",
            )
            .bind(&pat)
            .bind(&pat)
            .bind(&pat)
            .bind(&trimmed)
            .bind(format!("{trimmed}%"))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    pub async fn list_user_zen_packages(
        &self,
        owner_id: i64,
    ) -> Result<Vec<(ZenPackageSummaryRow, Vec<ZenPackageVersionRow>)>> {
        let pkgs = sqlx::query_as::<_, ZenPackageSummaryRow>(
            "SELECT p.id, p.name, p.owner_id,
                    COALESCE(u.username, 'desconocido') AS owner_username,
                    p.description, p.git_url, p.latest_version, p.downloads_count, p.updated_at
             FROM zen_packages p
             LEFT JOIN users u ON u.id = p.owner_id
             WHERE p.owner_id = ?
             ORDER BY p.updated_at DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(pkgs.len());
        for pkg in pkgs {
            let versions = self.list_zen_package_versions(pkg.id).await?;
            out.push((pkg, versions));
        }
        Ok(out)
    }

    pub async fn get_zen_package_detail(
        &self,
        name_lower: &str,
    ) -> Result<Option<ZenPackageDetailRow>> {
        let pkg = sqlx::query_as::<_, ZenPackageSummaryRow>(
            "SELECT p.id, p.name, p.owner_id,
                    COALESCE(u.username, 'desconocido') AS owner_username,
                    p.description, p.git_url, p.latest_version, p.downloads_count, p.updated_at
             FROM zen_packages p
             LEFT JOIN users u ON u.id = p.owner_id
             WHERE p.name = ?",
        )
        .bind(name_lower)
        .fetch_optional(&self.pool)
        .await?;
        let Some(package) = pkg else {
            return Ok(None);
        };
        let readme: String = sqlx::query_scalar("SELECT readme FROM zen_packages WHERE id = ?")
            .bind(package.id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or_default();
        let versions = self.list_zen_package_versions(package.id).await?;
        Ok(Some(ZenPackageDetailRow {
            package,
            readme,
            versions,
        }))
    }
}
