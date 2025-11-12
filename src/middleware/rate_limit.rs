//! Rate limiting middleware

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_window: u32,
    pub window_duration: Duration,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,
            window_duration: Duration::from_secs(60), // 1 minute
            burst_size: 10,
        }
    }
}

/// Auth-specific rate limit configurations
pub struct AuthRateLimits;

impl AuthRateLimits {
    pub const LOGIN: RateLimitConfig = RateLimitConfig {
        requests_per_window: 5,
        window_duration: Duration::from_secs(900), // 15 minutes
        burst_size: 2,
    };

    pub const REGISTER: RateLimitConfig = RateLimitConfig {
        requests_per_window: 3,
        window_duration: Duration::from_secs(3600), // 1 hour
        burst_size: 1,
    };

    pub const PASSWORD_RESET: RateLimitConfig = RateLimitConfig {
        requests_per_window: 3,
        window_duration: Duration::from_secs(3600), // 1 hour
        burst_size: 1,
    };

    pub const EMAIL_VERIFY: RateLimitConfig = RateLimitConfig {
        requests_per_window: 10,
        window_duration: Duration::from_secs(3600), // 1 hour
        burst_size: 2,
    };
}

/// Rate limiter state
#[derive(Debug)]
struct RateLimiterState {
    requests: HashMap<String, Vec<Instant>>,
}

impl RateLimiterState {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }

    fn is_allowed(&mut self, key: &str, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let window_start = now - config.window_duration;

        // Get or create request history for this key
        let requests = self.requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove expired requests
        requests.retain(|&timestamp| timestamp > window_start);

        // Check if under limit
        if requests.len() < config.requests_per_window as usize {
            requests.push(now);
            true
        } else {
            false
        }
    }

    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        let max_duration = Duration::from_secs(3600); // Keep up to 1 hour of history

        self.requests.retain(|_, requests| {
            requests.retain(|&timestamp| now.duration_since(timestamp) < max_duration);
            !requests.is_empty()
        });
    }
}

/// Rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    state: Arc<RwLock<RateLimiterState>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(RateLimiterState::new())),
        }
    }

    pub async fn is_allowed(&self, key: &str, config: &RateLimitConfig) -> bool {
        let mut state = self.state.write().await;
        state.is_allowed(key, config)
    }

    pub async fn cleanup(&self) {
        let mut state = self.state.write().await;
        state.cleanup_expired();
    }

    /// Get client IP address from request
    fn get_client_ip(req: &Request) -> String {
        // Try to get real IP from headers first
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // Take the first IP in the forwarded list
                return forwarded_str.split(',').next().unwrap_or("unknown").trim().to_string();
            }
        }

        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                return real_ip_str.to_string();
            }
        }

        if let Some(remote_addr) = req.extensions().get::<std::net::SocketAddr>() {
            return remote_addr.ip().to_string();
        }

        "unknown".to_string()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    rate_limiter: State<Arc<RateLimiter>>,
    config: State<RateLimitConfig>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = RateLimiter::get_client_ip(&request);
    let key = format!("{}:{}", client_ip, request.uri().path());

    if !rate_limiter.is_allowed(&key, &config).await {
        warn!(
            client_ip = %client_ip,
            path = %request.uri().path(),
            "Rate limit exceeded"
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Auth-specific rate limiting middleware
pub async fn auth_rate_limit_middleware(
    rate_limiter: State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    let config = match path {
        p if p.contains("/login") => &AuthRateLimits::LOGIN,
        p if p.contains("/register") => &AuthRateLimits::REGISTER,
        p if p.contains("/forgot-password") => &AuthRateLimits::PASSWORD_RESET,
        p if p.contains("/verify-email") => &AuthRateLimits::EMAIL_VERIFY,
        _ => return Ok(next.run(request).await),
    };

    let client_ip = RateLimiter::get_client_ip(&request);
    let key = format!("auth:{}:{}", client_ip, path);

    if !rate_limiter.is_allowed(&key, config).await {
        warn!(
            client_ip = %client_ip,
            path = %path,
            "Auth rate limit exceeded"
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Task to periodically clean up expired rate limit entries
pub async fn cleanup_task(rate_limiter: Arc<RateLimiter>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

    loop {
        interval.tick().await;
        rate_limiter.cleanup().await;
        tracing::debug!("Cleaned up expired rate limit entries");
    }
}