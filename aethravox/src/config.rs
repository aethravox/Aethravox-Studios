use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_addr: String,
    pub env: String,
    pub cookie_name: String,
    pub cookie_domain: String,
    pub public_url: String,
    pub log_level: String,
    pub owner_user: String,
    pub owner_email: String,
    pub owner_pass: String,
    pub production: bool,
    pub admin_unlock_cookie_name: String,
    pub admin_unlock_ttl_minutes: i64,
    // ── AethraMail (Sección 6): cliente mTLS del hub contra la API interna ──
    /// URL base de la API interna de AethraMail (HTTPS + mTLS).
    pub aethramail_api_url: String,
    /// Certs del cliente (generados por aethramail/scripts/gen_certs.sh).
    /// Paths relativos al CWD del hub (aethravox/): ../aethramail/certs/*.
    pub aethramail_client_cert: String,
    pub aethramail_client_key: String,
    pub aethramail_ca_cert: String,
    /// Dirección interna que el hub usa como remitente al mandar correos.
    pub aethramail_sender: String,
    /// Timeout (segundos) para cada llamada a la API interna.
    pub aethramail_timeout_secs: u64,
    // ── ZLibs / Zen Registry (GitHub REST API) ──
    pub github_token: String,
    pub github_org: String,
    pub github_owner: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            listen_addr: env_or("AETHRAVOX_LISTEN", "127.0.0.1:8080"),
            env: env_or("AETHRAVOX_ENV", "dev"),
            cookie_name: env_or("AETHRAVOX_COOKIE_NAME", "aethravox_session"),
            cookie_domain: env_or("AETHRAVOX_COOKIE_DOMAIN", ""),
            public_url: env_or("AETHRAVOX_PUBLIC_URL", "http://localhost:8080"),
            log_level: env_or("AETHRAVOX_LOG_LEVEL", "info"),
            owner_user: env_or("AETHRAVOX_OWNER_USER", "Aethravox"),
            owner_email: env_or("AETHRAVOX_OWNER_EMAIL", "owner@aethravox.local"),
            owner_pass: env_or("AETHRAVOX_OWNER_PASS", "cambiar_este_password_ya"),
            production: env_or("AETHRAVOX_ENV", "dev") == "prod",
            admin_unlock_cookie_name: env_or("AETHRAVOX_ADMIN_UNLOCK_COOKIE", "aethravox_admin_unlock"),
            // Tiempo de vida del desbloqueo de admin en minutos (default 30 min).
            // Pasado este tiempo sin actividad, el admin debe re-meter su password.
            admin_unlock_ttl_minutes: env::var("AETHRAVOX_ADMIN_UNLOCK_TTL_MIN")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(30),
            // AethraMail (Sección 6).
            aethramail_api_url: env_or("AETHRAMAIL_API_URL", "https://127.0.0.1:8449"),
            aethramail_client_cert: env_or("AETHRAMAIL_CLIENT_CERT", "./certs/client.pem"),
            aethramail_client_key: env_or("AETHRAMAIL_CLIENT_KEY", "./certs/client-key.pem"),
            aethramail_ca_cert: env_or("AETHRAMAIL_CA_CERT", "./certs/ca.pem"),
            aethramail_sender: env_or("AETHRAMAIL_SENDER", "noreply@aethra.sac"),
            aethramail_timeout_secs: env::var("AETHRAMAIL_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(10),
            github_token: env_or_alt("GITHUB_TOKEN", "AETHRAVOX_GITHUB_TOKEN", ""),
            github_org: env_or_alt("GITHUB_ORG", "AETHRAVOX_GITHUB_ORG", ""),
            github_owner: env_or_alt("GITHUB_OWNER", "AETHRAVOX_GITHUB_OWNER", ""),
        }
    }
}

fn env_or(key: &str, fallback: &str) -> String {
    env::var(key).unwrap_or_else(|_| fallback.to_string())
}

fn env_or_alt(primary: &str, secondary: &str, fallback: &str) -> String {
    env::var(primary)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var(secondary).ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| fallback.to_string())
}
