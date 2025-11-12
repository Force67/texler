//! Minimal OIDC test server to test GitHub OAuth integration

use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

#[derive(Clone)]
struct OidcTestState {
    oidc_client: authware::OidcClient,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment
    dotenvy::dotenv().ok();

    println!("🚀 Starting OIDC Test Server");

    // Load GitHub OAuth configuration
    let client_id = env::var("OIDC_PROVIDER_0_CLIENT_ID")
        .expect("OIDC_PROVIDER_0_CLIENT_ID not set");
    let client_secret = env::var("OIDC_PROVIDER_0_CLIENT_SECRET")
        .expect("OIDC_PROVIDER_0_CLIENT_SECRET not set");
    let redirect_uri = env::var("OIDC_PROVIDER_0_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:8080/api/v1/auth/oidc/callback".to_string());

    println!("📋 GitHub OAuth Configuration:");
    println!("  Client ID: {}...", &client_id[..8]);
    println!("  Client Secret: {}...", &client_secret[..8]);
    println!("  Redirect URI: {}", redirect_uri);

    // Create GitHub OAuth client
    let oidc_client = authware::OidcClient::builder()
        .new(authware::OidcProvider::GitHub, client_id, redirect_uri)
        .client_secret(client_secret)
        .scopes(vec!["user:email".to_string()])
        .pkce(true)
        .build()
        .map_err(|e| format!("Failed to create GitHub OAuth client: {}", e))?;

    println!("✅ GitHub OAuth client created successfully!");

    let app_state = Arc::new(OidcTestState { oidc_client });

    // Create router with OIDC endpoints
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/api/v1/auth/oidc/providers", get(get_oidc_providers))
        .route("/api/v1/auth/oidc/login", post(oidc_login))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("🌐 Server listening on http://localhost:8080");
    println!("\n🔗 Test URLs:");
    println!("  GET  http://localhost:8080/api/v1/auth/oidc/providers");
    println!("  POST http://localhost:8080/api/v1/auth/oidc/login");
    println!("\n📝 Example login request:");
    println!("  curl -X POST http://localhost:8080/api/v1/auth/oidc/login \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{\"provider\": \"github\"}'");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> ResponseJson<Value> {
    ResponseJson(json!({
        "status": "ok",
        "message": "OIDC Test Server is running",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn get_oidc_providers(State(_state): State<Arc<OidcTestState>>) -> ResponseJson<Value> {
    ResponseJson(json!({
        "success": true,
        "data": {
            "enabled": true,
            "providers": [
                {
                    "name": "github",
                    "display_name": "GitHub"
                }
            ]
        }
    }))
}

async fn oidc_login(
    State(state): State<Arc<OidcTestState>>,
    Json(payload): Value,
) -> Result<ResponseJson<Value>, StatusCode> {
    let provider = payload.get("provider")
        .and_then(|p| p.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    if provider != "github" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Generate authorization URL with PKCE
    match state.oidc_client.auth_url_with_pkce("user:email").await {
        Ok((auth_url, pkce_challenge, state_token)) => {
            println!("🔗 Generated auth URL for GitHub OAuth");
            Ok(ResponseJson(json!({
                "success": true,
                "data": {
                    "auth_url": auth_url.to_string(),
                    "state": state_token,
                    "pkce_challenge": Some(pkce_challenge),
                    "provider": "github"
                }
            })))
        }
        Err(e) => {
            eprintln!("❌ Failed to generate auth URL: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}