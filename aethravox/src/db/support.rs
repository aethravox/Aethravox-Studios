#![allow(unused_imports)]
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

use crate::models::{AuditEntry, BanInfo, Community, ConfigEntry, Conversation, ConversationRank, Document, IpBan, LinkedDevice, LinkCode, Message, PushSubscription, Role, SessionInfo, SuspensionInfo, Title, User, UserIpHistory, Web};

use super::Db;
use super::{hash_password, verify_password, random_hex, random_base32, random_numeric};

impl Db {
    pub async fn create_ticket(&self, user_id: i64, ticket_type: &str, subject: &str, body: &str) -> Result<i64> {
        let res = sqlx::query("INSERT INTO tickets (user_id, ticket_type, subject, body) VALUES (?, ?, ?, ?)")
            .bind(user_id).bind(ticket_type).bind(subject).bind(body)
            .execute(&self.pool).await?;
        Ok(res.last_insert_rowid())
    }

    /// Crea un ticket fileado por un invitado sin sesión. El dueño nominal es
    /// la cuenta sistema "invitado" (user_id pasada), pero guardamos la IP real
    /// del invitado en guest_ip para traceability + rate-limit. El admin puede
    /// distinguir tickets de invitado (guest_ip IS NOT NULL) de los normales.
    pub async fn create_guest_ticket(&self, user_id: i64, ticket_type: &str, subject: &str, body: &str, guest_ip: &str) -> Result<i64> {
        let res = sqlx::query("INSERT INTO tickets (user_id, ticket_type, subject, body, guest_ip) VALUES (?, ?, ?, ?, ?)")
            .bind(user_id).bind(ticket_type).bind(subject).bind(body).bind(guest_ip)
            .execute(&self.pool).await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn list_user_tickets(&self, user_id: i64) -> Result<Vec<crate::handlers::support::Ticket>> {
        let rows = sqlx::query_as::<_, crate::handlers::support::Ticket>(
            "SELECT id, user_id, ticket_type, subject, body, status, created_at, answered_at, reclamado_por, resolucion FROM tickets WHERE user_id = ? ORDER BY created_at DESC"
        ).bind(user_id).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn list_all_tickets(&self) -> Result<Vec<crate::handlers::support::Ticket>> {
        let rows = sqlx::query_as::<_, crate::handlers::support::Ticket>(
            "SELECT id, user_id, ticket_type, subject, body, status, created_at, answered_at, reclamado_por, resolucion FROM tickets ORDER BY created_at DESC"
        ).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn get_ticket(&self, id: i64) -> Result<Option<crate::handlers::support::Ticket>> {
        let row = sqlx::query_as::<_, crate::handlers::support::Ticket>(
            "SELECT id, user_id, ticket_type, subject, body, status, created_at, answered_at, reclamado_por, resolucion FROM tickets WHERE id = ?"
        ).bind(id).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    pub async fn add_ticket_message(&self, ticket_id: i64, from_user_id: i64, is_admin: i64, body: &str) -> Result<()> {
        sqlx::query("INSERT INTO ticket_messages (ticket_id, from_user_id, is_admin, body) VALUES (?, ?, ?, ?)")
            .bind(ticket_id).bind(from_user_id).bind(is_admin).bind(body)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_ticket_messages(&self, ticket_id: i64) -> Result<Vec<crate::handlers::support::TicketMessage>> {
        let rows = sqlx::query_as::<_, crate::handlers::support::TicketMessage>(
            "SELECT id, ticket_id, from_user_id, is_admin, body, created_at FROM ticket_messages WHERE ticket_id = ? ORDER BY created_at ASC"
        ).bind(ticket_id).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn update_ticket_status(&self, id: i64, status: &str) -> Result<()> {
        sqlx::query("UPDATE tickets SET status = ?, answered_at = CASE WHEN ? = 'answered' THEN ? ELSE answered_at END WHERE id = ?")
            .bind(status).bind(status).bind(Utc::now()).bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reclamar_ticket(&self, id: i64, admin_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE tickets SET reclamado_por = ?, status = 'answered', answered_at = COALESCE(answered_at, ?) WHERE id = ?",
        )
        .bind(admin_id).bind(Utc::now()).bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reclamar_ticket_if_unclaimed(&self, id: i64, admin_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE tickets SET reclamado_por = ? WHERE id = ? AND reclamado_por IS NULL",
        )
        .bind(admin_id).bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn terminar_ticket(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE tickets SET status = 'closed', resolucion = 'terminado', answered_at = COALESCE(answered_at, ?) WHERE id = ?")
            .bind(Utc::now()).bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn rechazar_ticket(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE tickets SET status = 'closed', resolucion = 'rechazado', answered_at = COALESCE(answered_at, ?) WHERE id = ?")
            .bind(Utc::now()).bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn cerrar_ticket(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE tickets SET status = 'closed', resolucion = NULL, answered_at = COALESCE(answered_at, ?) WHERE id = ?")
            .bind(Utc::now()).bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

}
