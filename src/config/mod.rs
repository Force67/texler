//! Configuration management for the Texler backend

use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, warn};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub websocket: WebSocketConfig,
    pub latex: LatexConfig,
    pub email: EmailConfig,
    pub features: FeaturesConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let config = Config {
            server: ServerConfig::load()?,
            database: DatabaseConfig::load()?,
            redis: RedisConfig::load()?,
            jwt: JwtConfig::load()?,
            websocket: WebSocketConfig::load()?,
            latex: LatexConfig::load()?,
            email: EmailConfig::load()?,
            features: FeaturesConfig::load()?,
            logging: LoggingConfig::load()?,
        };

        info!("Configuration loaded successfully");
        Ok(config)
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: usize,
    pub request_timeout: u64,
    pub keep_alive: u64,
    pub tls: Option<TlsConfig>,
}

impl ServerConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            workers: env::var("SERVER_WORKERS")
                .unwrap_or_else(|_| num_cpus::get().to_string())
                .parse()?,
            max_connections: env::var("SERVER_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()?,
            request_timeout: env::var("SERVER_REQUEST_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            keep_alive: env::var("SERVER_KEEP_ALIVE")
                .unwrap_or_else(|_| "75".to_string())
                .parse()?,
            tls: if env::var("SERVER_TLS_CERT").is_ok() {
                Some(TlsConfig {
                    cert_path: env::var("SERVER_TLS_CERT")?,
                    key_path: env::var("SERVER_TLS_KEY")?,
                })
            } else {
                None
            },
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
}

impl DatabaseConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(DatabaseConfig {
            host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("DATABASE_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()?,
            database: env::var("DATABASE_NAME")
                .unwrap_or_else(|_| "texler".to_string()),
            username: env::var("DATABASE_USER")
                .unwrap_or_else(|_| "postgres".to_string()),
            password: env::var("DATABASE_PASSWORD")
                .unwrap_or_else(|_| "".to_string()),
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()?,
            min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()?,
            connect_timeout: env::var("DATABASE_CONNECT_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            idle_timeout: env::var("DATABASE_IDLE_TIMEOUT")
                .unwrap_or_else(|_| "600".to_string())
                .parse()?,
        })
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database
        )
    }

    pub fn connection_url(&self) -> String {
        use std::str::FromStr;
        url::Url::parse(&format!(
            "postgres://{}:{}@{}:{}/{}",
            percent_encoding::utf8_percent_encode(&self.username, percent_encoding::NON_ALPHANUMERIC),
            percent_encoding::utf8_percent_encode(&self.password, percent_encoding::NON_ALPHANUMERIC),
            self.host,
            self.port,
            self.database
        ))
        .unwrap()
        .into()
    }
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

impl RedisConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(RedisConfig {
            url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            max_connections: env::var("REDIS_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            connection_timeout: env::var("REDIS_CONNECTION_TIMEOUT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()?,
        })
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: u64,
    pub refresh_expiration: u64,
    pub issuer: String,
}

impl JwtConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let secret = env::var("JWT_SECRET")?;

        if secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters long".into());
        }

        Ok(JwtConfig {
            secret,
            expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()?, // 24 hours in seconds
            refresh_expiration: env::var("JWT_REFRESH_EXPIRATION")
                .unwrap_or_else(|_| "604800".to_string())
                .parse()?, // 7 days in seconds
            issuer: env::var("JWT_ISSUER")
                .unwrap_or_else(|_| "texler".to_string()),
        })
    }
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub port: u16,
    pub max_connections: usize,
    pub heartbeat_interval: u64,
    pub message_size_limit: usize,
}

impl WebSocketConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(WebSocketConfig {
            port: env::var("WEBSOCKET_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()?,
            max_connections: env::var("WEBSOCKET_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,
            heartbeat_interval: env::var("WEBSOCKET_HEARTBEAT_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            message_size_limit: env::var("WEBSOCKET_MESSAGE_SIZE_LIMIT")
                .unwrap_or_else(|_| "65536".to_string())
                .parse()?,
        })
    }

    pub fn bind_address(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }
}

/// LaTeX compilation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatexConfig {
    pub timeout: u64,
    pub memory_limit: u64,
    pub output_size_limit: u64,
    pub temp_dir: String,
    pub engines: Vec<String>,
    pub default_engine: String,
}

impl LatexConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(LatexConfig {
            timeout: env::var("LATEX_TIMEOUT")
                .unwrap_or_else(|_| "30000".to_string())
                .parse()?, // 30 seconds
            memory_limit: env::var("LATEX_MEMORY_LIMIT")
                .unwrap_or_else(|_| "512".to_string())
                .parse()?, // 512 MB
            output_size_limit: env::var("LATEX_OUTPUT_SIZE_LIMIT")
                .unwrap_or_else(|_| "10485760".to_string())
                .parse()?, // 10 MB
            temp_dir: env::var("LATEX_TEMP_DIR")
                .unwrap_or_else(|_| "/tmp/texler".to_string()),
            engines: env::var("LATEX_ENGINES")
                .unwrap_or_else(|_| "pdflatex,xelatex,lualatex".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            default_engine: env::var("LATEX_DEFAULT_ENGINE")
                .unwrap_or_else(|_| "pdflatex".to_string()),
        })
    }
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub from_name: String,
}

impl EmailConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(EmailConfig {
            smtp_host: env::var("SMTP_HOST")
                .unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()?,
            smtp_username: env::var("SMTP_USERNAME")
                .unwrap_or_else(|_| "".to_string()),
            smtp_password: env::var("SMTP_PASSWORD")
                .unwrap_or_else(|_| "".to_string()),
            from_address: env::var("EMAIL_FROM_ADDRESS")
                .unwrap_or_else(|_| "noreply@texler.dev".to_string()),
            from_name: env::var("EMAIL_FROM_NAME")
                .unwrap_or_else(|_| "Texler".to_string()),
        })
    }
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub websocket: bool,
    pub collaboration: bool,
    pub search: bool,
    pub email: bool,
    pub latex_compilation: bool,
    pub file_storage: FileStorageConfig,
    pub rate_limiting: bool,
    pub metrics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    pub type_: String, // "local", "s3", "gcs"
    pub local_path: String,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
}

impl FeaturesConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(FeaturesConfig {
            websocket: env::var("FEATURE_WEBSOCKET")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            collaboration: env::var("FEATURE_COLLABORATION")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            search: env::var("FEATURE_SEARCH")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            email: env::var("FEATURE_EMAIL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
            latex_compilation: env::var("FEATURE_LATEX_COMPILATION")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            file_storage: FileStorageConfig {
                type_: env::var("FILE_STORAGE_TYPE")
                    .unwrap_or_else(|_| "local".to_string()),
                local_path: env::var("FILE_STORAGE_LOCAL_PATH")
                    .unwrap_or_else(|_| "./uploads".to_string()),
                s3_bucket: env::var("AWS_S3_BUCKET").ok(),
                s3_region: env::var("AWS_REGION").ok(),
            },
            rate_limiting: env::var("FEATURE_RATE_LIMITING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            metrics: env::var("FEATURE_METRICS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
        })
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "compact"
    pub file: Option<String>,
}

impl LoggingConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(LoggingConfig {
            level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            format: env::var("LOG_FORMAT")
                .unwrap_or_else(|_| "json".to_string()),
            file: env::var("LOG_FILE").ok(),
        })
    }
}