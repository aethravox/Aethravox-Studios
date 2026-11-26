#![allow(unused_imports)]
// Web 2 — Panel de administración (Owner + Admin).

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use tower_cookies::Cookies;

use crate::{
    auth_mw::AuthUser,
    models::{
        ActionOnUser, AdminCreateUserForm, BanForm, ChangeRoleForm, CreateWebForm,
        SuspendForm, UpdateConfigForm, User, WebPermsForm,
    },
    AppState,
};

// ===== Idea 1: re-autenticación para /admin/* =====


mod unlock;
pub use self::unlock::*;
mod overview;
pub use self::overview::*;
mod users_view;
pub use self::users_view::*;
mod user_actions;
pub use self::user_actions::*;
mod webs_view;
pub use self::webs_view::*;
mod web_custom;
pub use self::web_custom::*;
mod audit_view;
pub use self::audit_view::*;
mod config;
pub use self::config::*;
mod ip_ban_list;
pub use self::ip_ban_list::*;
mod ip_ban_api;
pub use self::ip_ban_api::*;
