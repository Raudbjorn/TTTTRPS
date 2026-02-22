//! Voice provider detection service
//!
//! Probes local endpoints to detect which self-hosted TTS services are available.
//! Cloud providers (ElevenLabs, OpenAI, FishAudio) require API keys and are not
//! detected here - they are always listed as options in the UI.

use reqwest::Client;
use std::time::Duration;
use chrono::Utc;

use super::types::{ProviderStatus, VoiceProviderDetection, VoiceProviderType};

const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);

/// Detect available self-hosted voice providers by probing their default endpoints.
///
/// Note: This only detects local/self-hosted providers. Cloud providers (ElevenLabs,
/// OpenAI TTS, Fish Audio) require API key validation and are handled separately.
pub async fn detect_providers() -> VoiceProviderDetection {
    let client = Client::builder()
        .timeout(DETECTION_TIMEOUT)
        .build()
        .expect("Failed to build HTTP client for provider detection");

    let providers_to_check = vec![
        VoiceProviderType::Ollama,
        VoiceProviderType::Chatterbox,
        VoiceProviderType::GptSoVits,
        VoiceProviderType::XttsV2,
        VoiceProviderType::FishSpeech,
        VoiceProviderType::Dia,
    ];

    let mut results = Vec::new();

    for provider in providers_to_check {
        let status = check_provider(&client, &provider).await;
        results.push(status);
    }

    VoiceProviderDetection {
        providers: results,
        detected_at: Some(Utc::now().to_rfc3339()),
    }
}

/// Check a single provider's availability
async fn check_provider(client: &Client, provider: &VoiceProviderType) -> ProviderStatus {
    let endpoint = match provider.default_endpoint() {
        Some(ep) => ep.to_string(),
        None => {
            return ProviderStatus {
                provider: provider.clone(),
                available: false,
                endpoint: None,
                version: None,
                error: Some("No default endpoint".to_string()),
            }
        }
    };

    match provider {
        VoiceProviderType::Ollama => check_ollama(client, &endpoint).await,
        VoiceProviderType::Chatterbox => check_chatterbox(client, &endpoint).await,
        VoiceProviderType::GptSoVits => check_gpt_sovits(client, &endpoint).await,
        VoiceProviderType::XttsV2 => check_xtts_v2(client, &endpoint).await,
        VoiceProviderType::FishSpeech => check_fish_speech(client, &endpoint).await,
        VoiceProviderType::Dia => check_dia(client, &endpoint).await,
        _ => ProviderStatus {
            provider: provider.clone(),
            available: false,
            endpoint: Some(endpoint),
            version: None,
            error: Some("Detection not implemented".to_string()),
        },
    }
}

/// Ollama: GET /api/tags or /api/version
async fn check_ollama(client: &Client, base_url: &str) -> ProviderStatus {
    let url = format!("{}/api/version", base_url);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let version = resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from));

            ProviderStatus {
                provider: VoiceProviderType::Ollama,
                available: true,
                endpoint: Some(base_url.to_string()),
                version,
                error: None,
            }
        }
        Ok(resp) => ProviderStatus {
            provider: VoiceProviderType::Ollama,
            available: false,
            endpoint: Some(base_url.to_string()),
            version: None,
            error: Some(format!("HTTP {}", resp.status())),
        },
        Err(e) => ProviderStatus {
            provider: VoiceProviderType::Ollama,
            available: false,
            endpoint: Some(base_url.to_string()),
            version: None,
            error: Some(connection_error(&e)),
        },
    }
}

/// Chatterbox: typically exposes a /health or root endpoint
async fn check_chatterbox(client: &Client, base_url: &str) -> ProviderStatus {
    check_provider_with_paths(
        client,
        base_url,
        VoiceProviderType::Chatterbox,
        &["/health", "/api/health", "/"],
    ).await
}

/// GPT-SoVITS: API at /tts or /
async fn check_gpt_sovits(client: &Client, base_url: &str) -> ProviderStatus {
    // GPT-SoVITS may return 405 on GET (expects POST) but still indicates it's running
    let paths = ["/", "/tts", "/api"];
    let mut last_error: Option<String> = None;

    for path in paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 405 => {
                return ProviderStatus {
                    provider: VoiceProviderType::GptSoVits,
                    available: true,
                    endpoint: Some(base_url.to_string()),
                    version: None,
                    error: None,
                };
            }
            Ok(resp) => {
                last_error = Some(format!("HTTP {} on {}", resp.status(), path));
            }
            Err(e) => {
                last_error = Some(connection_error(&e));
            }
        }
    }

    ProviderStatus {
        provider: VoiceProviderType::GptSoVits,
        available: false,
        endpoint: Some(base_url.to_string()),
        version: None,
        error: last_error.or_else(|| Some("No valid endpoint found".to_string())),
    }
}

/// XTTS-v2 (Coqui TTS server): /api/tts or /docs
///
/// Uses `check_provider_with_paths` for detailed error diagnostics:
/// - Connection refused vs HTTP error vs timeout
async fn check_xtts_v2(client: &Client, base_url: &str) -> ProviderStatus {
    check_provider_with_paths(
        client,
        base_url,
        VoiceProviderType::XttsV2,
        &["/docs", "/", "/api/tts"],
    ).await
}

/// Fish Speech: /v1/tts or /health
///
/// Uses `check_provider_with_paths` for detailed error diagnostics.
async fn check_fish_speech(client: &Client, base_url: &str) -> ProviderStatus {
    check_provider_with_paths(
        client,
        base_url,
        VoiceProviderType::FishSpeech,
        &["/health", "/v1/health", "/"],
    ).await
}

/// Dia: /health or /api/health
///
/// Uses `check_provider_with_paths` for detailed error diagnostics.
async fn check_dia(client: &Client, base_url: &str) -> ProviderStatus {
    check_provider_with_paths(
        client,
        base_url,
        VoiceProviderType::Dia,
        &["/health", "/api/health", "/"],
    ).await
}

/// Generic provider check that tries multiple paths and returns detailed errors.
///
/// Distinguishes between:
/// - Connection refused → "Not running (connection refused)"
/// - HTTP error → "HTTP {status} on {path}"
/// - Timeout → "Timeout (service may be slow or unresponsive)"
///
/// Only marks provider as available if a successful HTTP response is received.
async fn check_provider_with_paths(
    client: &Client,
    base_url: &str,
    provider: VoiceProviderType,
    paths: &[&str],
) -> ProviderStatus {
    let mut last_error: Option<String> = None;

    for path in paths {
        let url = format!("{}{}", base_url, path);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return ProviderStatus {
                    provider,
                    available: true,
                    endpoint: Some(base_url.to_string()),
                    version: None,
                    error: None,
                };
            }
            Ok(resp) => {
                // Got a response but not success - record the status
                last_error = Some(format!("HTTP {} on {}", resp.status(), path));
            }
            Err(e) => {
                last_error = Some(connection_error(&e));
            }
        }
    }

    ProviderStatus {
        provider,
        available: false,
        endpoint: Some(base_url.to_string()),
        version: None,
        error: last_error.or_else(|| Some("No valid endpoint found".to_string())),
    }
}

/// Convert reqwest error to user-friendly message
fn connection_error(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "Not running (connection refused)".to_string()
    } else if e.is_timeout() {
        "Timeout (service may be slow or unresponsive)".to_string()
    } else {
        format!("Network error: {}", e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_providers_returns_all() {
        let detection = detect_providers().await;
        // Should return status for all local providers
        assert!(detection.providers.len() >= 5);
        assert!(detection.detected_at.is_some());
    }
}
