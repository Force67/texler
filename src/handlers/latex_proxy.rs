//! LaTeX compilation proxy handler
//!
//! This module provides a simple proxy to the LaTeX compilation service
//! for development and testing purposes.

use crate::error::AppError;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::server::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LaTeX compilation request (matching the frontend's expected format)
#[derive(Debug, Deserialize)]
pub struct LatexCompileRequest {
    pub files: HashMap<String, String>,
    pub main_file: String,
}

/// LaTeX compilation response (matching the existing Python service format)
#[derive(Debug, Serialize)]
pub struct LatexCompileResponse {
    pub success: bool,
    pub output: String,
    pub errors: String,
    pub pdf: Option<String>,
    pub log: Option<String>,
    pub parsed_errors: Vec<serde_json::Value>,
}

/// Proxy LaTeX compilation requests to the LaTeX service
pub async fn compile_latex(
    State(state): State<AppState>,
    Json(payload): Json<LatexCompileRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Get the LaTeX service URL from environment or config
    let latex_service_url = std::env::var("LATEX_SERVICE_URL")
        .unwrap_or_else(|_| "http://latex:8081".to_string());

    let client = reqwest::Client::new();

    // Forward the request to the LaTeX service
    let response = client
        .post(format!("{}/compile", latex_service_url))
        .json(&serde_json::json!({
            "files": payload.files,
            "mainFile": payload.main_file
        }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to LaTeX service: {}", e)))?;

    if response.status().is_success() {
        let latex_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse LaTeX service response: {}", e)))?;

        Ok(Json(latex_response))
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        Err(AppError::Internal(format!("LaTeX service error: {}", error_text)))
    }
}

/// Health check for LaTeX proxy
pub async fn latex_health_check() -> Result<impl IntoResponse, AppError> {
    let latex_service_url = std::env::var("LATEX_SERVICE_URL")
        .unwrap_or_else(|_| "http://latex:8081".to_string());

    let client = reqwest::Client::new();

    match client.get(format!("{}/health", latex_service_url)).send().await {
        Ok(response) if response.status().is_success() => {
            Ok(Json(serde_json::json!({
                "status": "ok",
                "latex_service": "connected"
            })))
        }
        Ok(_) => Err(AppError::Internal("LaTeX service is unhealthy".to_string())),
        Err(e) => Err(AppError::Internal(format!("Failed to connect to LaTeX service: {}", e))),
    }
}