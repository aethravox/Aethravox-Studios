#![allow(unused_imports)]
use axum::{
    extract::{Form, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    auth_mw::get_client_ip,
    db::verify_password,
    models::{LoginForm, RecoverForm, RegisterForm, ResetForm},
    templates::*,
    AppState,
};

// ===== GET / =====

mod login;
pub use self::login::*;
mod account;
pub use self::account::*;
