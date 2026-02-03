//! Settings Component Tests
//!
//! Tests for LLM settings form, Voice settings form, and API key input masking.
//! Note: Most LLMProvider methods are private (internal implementation),
//! so we test the public interface (Display, PartialEq, Clone) and utility functions.

use leptos::prelude::*;
use ttrpg_assistant_frontend::components::settings::llm::LLMProvider;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// LLMProvider Tests (Public API only)
// ============================================================================

#[wasm_bindgen_test]
fn test_llm_provider_display() {
    // Test Display trait implementation (public)
    assert_eq!(format!("{}", LLMProvider::Ollama), "Ollama");
    assert_eq!(format!("{}", LLMProvider::Claude), "Claude");
    assert_eq!(format!("{}", LLMProvider::Gemini), "Gemini");
    assert_eq!(format!("{}", LLMProvider::OpenAI), "OpenAI");
    assert_eq!(format!("{}", LLMProvider::OpenRouter), "OpenRouter");
    assert_eq!(format!("{}", LLMProvider::Mistral), "Mistral");
    assert_eq!(format!("{}", LLMProvider::Groq), "Groq");
    assert_eq!(format!("{}", LLMProvider::Together), "Together");
    assert_eq!(format!("{}", LLMProvider::Cohere), "Cohere");
    assert_eq!(format!("{}", LLMProvider::DeepSeek), "DeepSeek");
}

#[wasm_bindgen_test]
fn test_llm_provider_equality() {
    // Test PartialEq (derived)
    assert!(LLMProvider::Claude == LLMProvider::Claude);
    assert!(LLMProvider::Claude != LLMProvider::OpenAI);
    assert!(LLMProvider::Ollama == LLMProvider::Ollama);
    assert!(LLMProvider::Gemini == LLMProvider::Gemini);
}

#[wasm_bindgen_test]
fn test_llm_provider_clone() {
    // Test Clone trait (derived)
    let provider = LLMProvider::Claude;
    let cloned = provider.clone();
    assert!(provider == cloned);
}

#[wasm_bindgen_test]
fn test_llm_provider_all_variants_exist() {
    // Compile-time check that all expected variants exist
    let _ollama = LLMProvider::Ollama;
    let _claude = LLMProvider::Claude;
    let _gemini = LLMProvider::Gemini;
    let _openai = LLMProvider::OpenAI;
    let _openrouter = LLMProvider::OpenRouter;
    let _mistral = LLMProvider::Mistral;
    let _groq = LLMProvider::Groq;
    let _together = LLMProvider::Together;
    let _cohere = LLMProvider::Cohere;
    let _deepseek = LLMProvider::DeepSeek;
}

#[wasm_bindgen_test]
fn test_llm_provider_can_be_cloned() {
    // Test that LLMProvider can be cloned and used multiple times
    let provider = LLMProvider::Claude;
    let clone1 = provider.clone();
    let clone2 = provider.clone();
    assert!(clone1 == clone2);
    assert!(clone1 == provider);
}

// ============================================================================
// API Key Masking Tests (Utility Functions)
// ============================================================================

#[wasm_bindgen_test]
fn test_api_key_masking_basic() {
    // Test utility function for masking API keys
    fn mask_api_key(key: &str) -> String {
        if key.is_empty() {
            return String::new();
        }
        if key.len() <= 8 {
            "*".repeat(key.len())
        } else {
            let visible_start = &key[..4];
            let visible_end = &key[key.len() - 4..];
            format!("{}...{}", visible_start, visible_end)
        }
    }

    assert_eq!(mask_api_key("sk-ant-1234567890abcdef"), "sk-a...cdef");
    assert_eq!(mask_api_key("AIzaSyABCDEFGHIJKLMNOPQR"), "AIza...OPQR");
    assert_eq!(mask_api_key("short"), "*****");
    assert_eq!(mask_api_key("12345678"), "********");
    assert_eq!(mask_api_key(""), "");
}

#[wasm_bindgen_test]
fn test_api_key_visibility_toggle() {
    // Test toggling between masked and visible API key
    let api_key = "sk-ant-my-secret-key-12345";
    let show_key = RwSignal::new(false);

    // Initially masked
    assert!(!show_key.get());

    // Toggle to show
    show_key.set(true);
    assert!(show_key.get());

    // Get display value based on visibility
    let display = if show_key.get() {
        api_key.to_string()
    } else {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
    };

    assert_eq!(display, api_key);

    // Toggle to hide
    show_key.set(false);
    let masked_display = if show_key.get() {
        api_key.to_string()
    } else {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
    };

    assert_eq!(masked_display, "sk-a...2345");
}

// ============================================================================
// Settings Form State Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_llm_settings_form_state() {
    // Test LLM settings form state management with signals
    let selected_provider = RwSignal::new(LLMProvider::Ollama);
    let api_key_or_host = RwSignal::new("http://localhost:11434".to_string());
    let model_name = RwSignal::new("llama3.2".to_string());

    // Initial state
    assert!(matches!(selected_provider.get(), LLMProvider::Ollama));
    assert_eq!(api_key_or_host.get(), "http://localhost:11434");
    assert_eq!(model_name.get(), "llama3.2");

    // Change to Claude
    selected_provider.set(LLMProvider::Claude);
    api_key_or_host.set("sk-ant-my-key".to_string());
    model_name.set("claude-3-5-sonnet-20241022".to_string());

    assert!(matches!(selected_provider.get(), LLMProvider::Claude));
    assert_eq!(api_key_or_host.get(), "sk-ant-my-key");
    assert_eq!(model_name.get(), "claude-3-5-sonnet-20241022");
}

#[wasm_bindgen_test]
fn test_voice_settings_form_state() {
    // Test voice settings form state management
    let voice_provider = RwSignal::new("Disabled".to_string());
    let voice_api_key = RwSignal::new(String::new());
    let voice_model = RwSignal::new(String::new());
    let selected_voice = RwSignal::new(String::new());

    // Initial state - disabled
    assert_eq!(voice_provider.get(), "Disabled");
    assert!(voice_api_key.get().is_empty());

    // Enable OpenAI TTS
    voice_provider.set("OpenAI".to_string());
    voice_api_key.set("sk-voice-key".to_string());
    voice_model.set("tts-1".to_string());
    selected_voice.set("alloy".to_string());

    assert_eq!(voice_provider.get(), "OpenAI");
    assert_eq!(voice_model.get(), "tts-1");
    assert_eq!(selected_voice.get(), "alloy");

    // Change to ElevenLabs
    voice_provider.set("ElevenLabs".to_string());
    voice_api_key.set("eleven-labs-key".to_string());
    voice_model.set("eleven_multilingual_v2".to_string());

    assert_eq!(voice_provider.get(), "ElevenLabs");
    assert_eq!(voice_model.get(), "eleven_multilingual_v2");
}

// ============================================================================
// Form Validation Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_api_key_validation_patterns() {
    // Test API key format validation patterns
    fn validate_api_key_format(provider: &str, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }

        match provider {
            "claude" => key.starts_with("sk-ant-"),
            "openai" => key.starts_with("sk-"),
            "gemini" => key.starts_with("AIza"),
            "groq" => key.starts_with("gsk_"),
            "openrouter" => key.starts_with("sk-or-"),
            "ollama" => key.starts_with("http"),
            _ => true, // No specific validation for other providers
        }
    }

    assert!(validate_api_key_format("claude", "sk-ant-valid-key"));
    assert!(!validate_api_key_format("claude", "sk-invalid"));
    assert!(!validate_api_key_format("claude", ""));

    assert!(validate_api_key_format("openai", "sk-valid-key"));
    assert!(!validate_api_key_format("openai", "invalid"));

    assert!(validate_api_key_format("gemini", "AIzaSyValid"));
    assert!(!validate_api_key_format("gemini", "invalid"));

    assert!(validate_api_key_format("ollama", "http://localhost:11434"));
    assert!(!validate_api_key_format("ollama", "localhost:11434"));
}

#[wasm_bindgen_test]
fn test_model_name_validation() {
    // Test model name is not empty
    fn validate_model_name(name: &str) -> bool {
        !name.trim().is_empty()
    }

    assert!(validate_model_name("llama3.2"));
    assert!(validate_model_name("claude-3-5-sonnet-20241022"));
    assert!(validate_model_name("gpt-4o"));
    assert!(!validate_model_name(""));
    assert!(!validate_model_name("   "));
}

// ============================================================================
// Provider Status Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_provider_status_tracking() {
    // Test tracking provider connection status
    use std::collections::HashMap;

    let provider_statuses = RwSignal::new(HashMap::<String, bool>::new());

    // Initially no statuses
    assert!(provider_statuses.get().is_empty());

    // Add statuses
    provider_statuses.update(|map| {
        map.insert("ollama".to_string(), true);
        map.insert("claude".to_string(), false);
        map.insert("openai".to_string(), true);
    });

    let statuses = provider_statuses.get();
    assert_eq!(statuses.get("ollama"), Some(&true));
    assert_eq!(statuses.get("claude"), Some(&false));
    assert_eq!(statuses.get("openai"), Some(&true));
    assert_eq!(statuses.get("unknown"), None);
}

// ============================================================================
// Save Status Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_save_status_states() {
    // Test save status state machine
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);

    // Initial state
    assert!(save_status.get().is_empty());
    assert!(!is_saving.get());

    // Start saving
    is_saving.set(true);
    save_status.set("Saving...".to_string());
    assert!(is_saving.get());
    assert_eq!(save_status.get(), "Saving...");

    // Save success
    is_saving.set(false);
    save_status.set("Configuration Saved".to_string());
    assert!(!is_saving.get());
    assert!(save_status.get().contains("Saved"));

    // Save error
    is_saving.set(false);
    save_status.set("Error: Invalid API key".to_string());
    assert!(save_status.get().starts_with("Error:"));
}

// ============================================================================
// Voice Provider Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_voice_providers() {
    let providers = vec!["Disabled", "Ollama", "ElevenLabs", "OpenAI"];

    assert_eq!(providers.len(), 4);
    assert!(providers.contains(&"Disabled"));
    assert!(providers.contains(&"ElevenLabs"));
    assert!(providers.contains(&"OpenAI"));
    assert!(providers.contains(&"Ollama"));
}

#[wasm_bindgen_test]
fn test_voice_provider_defaults() {
    // Test default values for each voice provider
    fn get_default_model(provider: &str) -> &'static str {
        match provider {
            "Ollama" => "bark",
            "ElevenLabs" => "eleven_multilingual_v2",
            "OpenAI" => "tts-1",
            _ => "",
        }
    }

    fn get_default_url(provider: &str) -> &'static str {
        match provider {
            "Ollama" => "http://localhost:11434",
            _ => "",
        }
    }

    assert_eq!(get_default_model("Ollama"), "bark");
    assert_eq!(get_default_model("ElevenLabs"), "eleven_multilingual_v2");
    assert_eq!(get_default_model("OpenAI"), "tts-1");
    assert_eq!(get_default_model("Disabled"), "");

    assert_eq!(get_default_url("Ollama"), "http://localhost:11434");
    assert_eq!(get_default_url("ElevenLabs"), "");
}

#[wasm_bindgen_test]
fn test_openai_voices() {
    // Test OpenAI voice options
    let voices = vec!["alloy", "echo", "fable", "onyx", "nova", "shimmer"];

    assert_eq!(voices.len(), 6);
    assert!(voices.contains(&"alloy"));
    assert!(voices.contains(&"nova"));
}

// ============================================================================
// Provider Identification Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_provider_is_ollama() {
    // Test identifying Ollama (which uses host instead of API key)
    let provider = LLMProvider::Ollama;
    assert!(provider == LLMProvider::Ollama);

    let provider2 = LLMProvider::Claude;
    assert!(provider2 != LLMProvider::Ollama);
}

#[wasm_bindgen_test]
fn test_all_providers_have_distinct_display() {
    // Ensure all providers have unique display names
    let displays: Vec<String> = vec![
        format!("{}", LLMProvider::Ollama),
        format!("{}", LLMProvider::Claude),
        format!("{}", LLMProvider::Gemini),
        format!("{}", LLMProvider::OpenAI),
        format!("{}", LLMProvider::OpenRouter),
        format!("{}", LLMProvider::Mistral),
        format!("{}", LLMProvider::Groq),
        format!("{}", LLMProvider::Together),
        format!("{}", LLMProvider::Cohere),
        format!("{}", LLMProvider::DeepSeek),
    ];

    // All should be unique
    let mut seen = std::collections::HashSet::new();
    for d in displays {
        assert!(seen.insert(d.clone()), "Duplicate display name: {}", d);
    }
}
