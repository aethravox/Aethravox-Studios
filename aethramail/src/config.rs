use std::env;

/// Config de AethraMail cargada desde .env (ver .env.example).
#[derive(Clone, Debug)]
pub struct Config {
    /// Webmail UI (HTTP, navegador).
    pub http_addr: String,
    /// API interna (HTTPS + mTLS, solo el hub).
    pub api_addr: String,
    /// Path a la DB SQLite propia.
    pub db_path: String,
    /// Dominios internos disponibles al crear cuenta (5.3).
    pub domains: Vec<String>,
    /// Certs mTLS (5.8).
    pub tls_server_cert: String,
    pub tls_server_key: String,
    pub tls_client_ca: String,
    /// Rate limit API interna: envíos por minuto por (client cert / from).
    pub api_rate_per_min: u32,
    pub cookie_name: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        let domains: Vec<String> = env_or("AETHRAMAIL_DOMAINS", "shadow.sac,aethra.sac,cents.sac")
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            http_addr: env_or("AETHRAMAIL_HTTP_ADDR", "127.0.0.1:8088"),
            api_addr: env_or("AETHRAMAIL_API_ADDR", "127.0.0.1:8449"),
            db_path: env_or("AETHRAMAIL_DB", "data/aethramail.db"),
            domains,
            tls_server_cert: env_or("AETHRAMAIL_TLS_SERVER_CERT", "certs/server.pem"),
            tls_server_key: env_or("AETHRAMAIL_TLS_SERVER_KEY", "certs/server-key.pem"),
            tls_client_ca: env_or("AETHRAMAIL_TLS_CLIENT_CA", "certs/ca.pem"),
            api_rate_per_min: env_or("AETHRAMAIL_API_RATE_PER_MIN", "60")
                .parse()
                .unwrap_or(60),
            cookie_name: env_or("AETHRAMAIL_COOKIE_NAME", "aethramail_session"),
            log_level: env_or("AETHRAMAIL_LOG_LEVEL", "info"),
        }
    }

    /// ¿`domain` es uno de los dominios internos permitidos?
    pub fn is_allowed_domain(&self, domain: &str) -> bool {
        self.domains.iter().any(|d| d == domain)
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
