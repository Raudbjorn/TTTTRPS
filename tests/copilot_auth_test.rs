use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};
use ttrpg_assistant::oauth::copilot::CopilotClient;
use ttrpg_assistant::oauth::copilot::models::auth::TokenInfo;
use ttrpg_assistant::oauth::copilot::storage::MemoryTokenStorage;
use ttrpg_assistant::oauth::copilot::config::CopilotConfig;
use chrono::Utc;

#[tokio::test]
async fn test_copilot_token_auto_refresh() {
    let mock_server = MockServer::start().await;

    // 1. Initial token that is about to expire (within 60s buffer)
    let initial_expires_at = Utc::now().timestamp() + 30;
    let initial_token = TokenInfo::with_copilot(
        "gho_initial_github_token",
        "cop_initial_copilot_token",
        initial_expires_at
    );

    let storage = MemoryTokenStorage::with_token(initial_token);

    // 2. Mock the refresh endpoint
    let new_expires_at = Utc::now().timestamp() + 3600;
    let refresh_response = serde_json::json!({
        "token": "cop_refreshed_copilot_token",
        "expires_at": new_expires_at,
        "refresh_in": 1800
    });

    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .and(header("authorization", "token gho_initial_github_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(refresh_response))
        .mount(&mock_server)
        .await;

    // 3. Setup client with mock server URL
    let config = CopilotConfig::default()
        .with_api_base_url(mock_server.uri()) // Use mock server for API calls
        .with_auto_refresh(true);

    // We also need to override the TOKEN_URL used by exchange_for_copilot_token
    // but the client uses exchange_config which takes it from COPILOT_TOKEN_URL constant or similar.
    // Wait, CopilotConfig doesn't seem to have token_url.
    // Let's check CopilotConfig in src-tauri/src/gate/copilot/config.rs
}
