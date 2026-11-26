use anyhow::Result;
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;
use sqlx::sqlite::SqlitePool;

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct EmailAccount {
    pub id: i64,
    pub address: String,
    pub domain: String,
    #[serde(skip)]
    pub password_hash: String,
    pub created_at: String,
}

impl EmailAccount {
    /// Dirección completa: "alice@shadow.sac".
    pub fn full(&self) -> String {
        format!("{}@{}", self.address, self.domain)
    }
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Email {
    pub id: i64,
    pub from_address: String,
    pub to_address: String,
    pub subject: String,
    pub body: String,
    pub folder: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

/// Trocea "alice@shadow.sac" en ("alice", "shadow.sac").
/// Devuelve None si no tiene exactamente un '@' o el local está vacío.
pub fn split_address(full: &str) -> Option<(&str, &str)> {
    let full = full.trim();
    let mut parts = full.splitn(2, '@');
    let local = parts.next()?.trim();
    let domain = parts.next()?.trim();
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    Some((local, domain))
}

pub async fn create_account(
    pool: &SqlitePool,
    address: &str,
    domain: &str,
    password_hash: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO email_accounts (address, domain, password_hash) VALUES (?, ?, ?)",
    )
    .bind(address)
    .bind(domain)
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_account_by_full(
    pool: &SqlitePool,
    address: &str,
    domain: &str,
) -> Result<Option<EmailAccount>> {
    let acc = sqlx::query_as::<_, EmailAccount>(
        "SELECT id, address, domain, password_hash, created_at \
         FROM email_accounts WHERE address = ? AND domain = ?",
    )
    .bind(address)
    .bind(domain)
    .fetch_optional(pool)
    .await?;
    Ok(acc)
}

pub async fn find_account_by_id(pool: &SqlitePool, id: i64) -> Result<Option<EmailAccount>> {
    let acc = sqlx::query_as::<_, EmailAccount>(
        "SELECT id, address, domain, password_hash, created_at \
         FROM email_accounts WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(acc)
}

/// ¿Existe una cuenta con esa dirección completa? (validación de destinatario, 5.9)
pub async fn account_exists(pool: &SqlitePool, address: &str, domain: &str) -> Result<bool> {
    Ok(find_account_by_full(pool, address, domain).await?.is_some())
}

pub async fn insert_email(
    pool: &SqlitePool,
    from_address: &str,
    to_address: &str,
    subject: &str,
    body: &str,
    folder: &str,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO emails (from_address, to_address, subject, body, folder) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(from_address)
    .bind(to_address)
    .bind(subject)
    .bind(body)
    .bind(folder)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn list_emails(
    pool: &SqlitePool,
    owner: &str,
    folder: &str,
) -> Result<Vec<Email>> {
    // Para 'inbox'/'trash' filtramos por to_address; para 'sent' por from_address.
    let rows = if folder == "sent" {
        sqlx::query_as::<_, Email>(
            "SELECT id, from_address, to_address, subject, body, folder, read_at, created_at \
             FROM emails WHERE from_address = ? AND folder = 'sent' \
             ORDER BY created_at DESC",
        )
        .bind(owner)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Email>(
            "SELECT id, from_address, to_address, subject, body, folder, read_at, created_at \
             FROM emails WHERE to_address = ? AND folder = ? \
             ORDER BY created_at DESC",
        )
        .bind(owner)
        .bind(folder)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

pub async fn get_email(pool: &SqlitePool, id: i64) -> Result<Option<Email>> {
    let row = sqlx::query_as::<_, Email>(
        "SELECT id, from_address, to_address, subject, body, folder, read_at, created_at \
         FROM emails WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn mark_read(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("UPDATE emails SET read_at = datetime('now') WHERE id = ? AND read_at IS NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn count_unread(pool: &SqlitePool, owner: &str) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM emails WHERE to_address = ? AND folder = 'inbox' AND read_at IS NULL",
    )
    .bind(owner)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Para logs: parsea el created_at "YYYY-MM-DD HH:MM:SS" UTC.
pub fn parse_ts(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
}
