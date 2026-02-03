//! AI Conversation Panel
//!
//! Chat interface for AI assistance during campaign creation.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::services::wizard_state::use_wizard_context;

// ============================================================================
// Types
// ============================================================================

/// Chat message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub is_streaming: bool,
    pub suggestions: Vec<Suggestion>,
    pub citations: Vec<Citation>,
}

/// Suggestion chip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub data: Option<String>,
}

/// Suggestion type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    Accept,
    Edit,
    Reject,
    Expand,
}

/// Citation reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub source: String,
    pub page: Option<u32>,
    pub excerpt: Option<String>,
}

// ============================================================================
// Components
// ============================================================================

/// Single message bubble
#[component]
fn MessageBubble(
    message: ChatMessage,
    #[prop(optional)] on_suggestion_click: Option<Callback<Suggestion>>,
    #[prop(optional)] on_citation_click: Option<Callback<Citation>>,
) -> impl IntoView {
    let is_user = message.role == MessageRole::User;
    let is_streaming = message.is_streaming;
    let has_suggestions = !message.suggestions.is_empty();
    let has_citations = !message.citations.is_empty();
    let citations = message.citations.clone();

    view! {
        <div class=format!(
            "flex {} mb-4",
            if is_user { "justify-end" } else { "justify-start" }
        )>
            <div class=format!(
                "max-w-[85%] rounded-lg px-4 py-3 {}",
                if is_user {
                    "bg-purple-600 text-white"
                } else {
                    "bg-zinc-800 text-zinc-100"
                }
            )>
                // Message content
                <div class="prose prose-invert prose-sm max-w-none">
                    {message.content.clone()}
                    {is_streaming.then(|| view! {
                        <span class="inline-block w-2 h-4 bg-purple-400 animate-pulse ml-1" />
                    })}
                </div>

                // Suggestions
                {has_suggestions.then(|| {
                    let suggestions = message.suggestions.clone();
                    view! {
                        <div class="mt-3 pt-3 border-t border-zinc-700/50">
                            <div class="flex flex-wrap gap-2">
                                {suggestions.iter().map(|s| {
                                    let suggestion = s.clone();
                                    let btn_class = match s.suggestion_type {
                                        SuggestionType::Accept => "bg-green-900/50 text-green-300 hover:bg-green-900",
                                        SuggestionType::Edit => "bg-blue-900/50 text-blue-300 hover:bg-blue-900",
                                        SuggestionType::Reject => "bg-red-900/50 text-red-300 hover:bg-red-900",
                                        SuggestionType::Expand => "bg-purple-900/50 text-purple-300 hover:bg-purple-900",
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=format!("px-3 py-1 text-xs rounded-full transition-colors {}", btn_class)
                                            on:click={
                                                let sug = suggestion.clone();
                                                move |_| {
                                                    if let Some(cb) = on_suggestion_click {
                                                        cb.run(sug.clone());
                                                    }
                                                }
                                            }
                                        >
                                            {s.text.clone()}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                })}

                // Citations
                {has_citations.then(move || view! {
                    <div class="mt-2 text-xs text-zinc-500">
                        {citations.iter().map(|c| {
                            let citation = c.clone();
                            view! {
                                <button
                                    type="button"
                                    class="inline-flex items-center gap-1 px-2 py-0.5 bg-zinc-900/50 rounded hover:bg-zinc-900 transition-colors mr-1 mb-1"
                                    on:click={
                                        let cit = citation.clone();
                                        move |_| {
                                            if let Some(cb) = on_citation_click {
                                                cb.run(cit.clone());
                                            }
                                        }
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                    </svg>
                                    {c.source.clone()}
                                    {c.page.map(|p| format!(" p.{}", p))}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                })}
            </div>
        </div>
    }
}

/// Input area component
#[component]
fn MessageInput(on_send: Callback<String>, is_sending: Signal<bool>) -> impl IntoView {
    let input_value = RwSignal::new(String::new());

    let handle_send = move |_: leptos::ev::MouseEvent| {
        let msg = input_value.get();
        if !msg.trim().is_empty() {
            on_send.run(msg);
            input_value.set(String::new());
        }
    };

    let handle_keypress = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let msg = input_value.get();
            if !msg.trim().is_empty() {
                on_send.run(msg);
                input_value.set(String::new());
            }
        }
    };

    view! {
        <div class="p-4 border-t border-zinc-800">
            <div class="flex gap-2">
                <textarea
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                    rows="2"
                    placeholder="Ask the AI for help..."
                    prop:value=move || input_value.get()
                    on:input=move |ev| input_value.set(event_target_value(&ev))
                    on:keypress=handle_keypress
                    disabled=move || is_sending.get()
                />
                <button
                    type="button"
                    class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors
                           disabled:opacity-50 disabled:cursor-not-allowed self-end"
                    disabled=move || is_sending.get() || input_value.get().trim().is_empty()
                    on:click=handle_send
                >
                    {move || if is_sending.get() {
                        view! {
                            <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                            </svg>
                        }.into_any()
                    } else {
                        view! {
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                            </svg>
                        }.into_any()
                    }}
                </button>
            </div>
            <QuickPrompts input_value=input_value />
        </div>
    }
}

/// Quick prompt buttons
#[component]
fn QuickPrompts(input_value: RwSignal<String>) -> impl IntoView {
    let prompts = [
        "Suggest themes",
        "Help with NPCs",
        "Story ideas",
        "Encounter tips",
    ];

    view! {
        <div class="flex flex-wrap gap-1 mt-2">
            {prompts.iter().map(|prompt| {
                let prompt_text = prompt.to_string();
                view! {
                    <button
                        type="button"
                        class="px-2 py-1 text-xs bg-zinc-800 text-zinc-400 rounded hover:bg-zinc-700 hover:text-zinc-300 transition-colors"
                        on:click={
                            let pt = prompt_text.clone();
                            move |_| input_value.set(pt.clone())
                        }
                    >
                        {*prompt}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}

/// Context info header
#[component]
fn ContextHeader() -> impl IntoView {
    let ctx = use_wizard_context();

    view! {
        <div class="p-3 border-b border-zinc-800 bg-zinc-900/50">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                    <svg class="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                    </svg>
                    <span class="text-sm font-medium text-white">"AI Assistant"</span>
                </div>
                <span class="text-xs text-zinc-500">
                    "Step: "{move || ctx.current_step.get().label()}
                </span>
            </div>
        </div>
    }
}

/// Empty state
#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center p-6">
            <div class="text-center max-w-xs">
                <svg class="w-12 h-12 text-zinc-600 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                </svg>
                <h4 class="text-zinc-300 font-medium mb-2">"AI Assistant Ready"</h4>
                <p class="text-zinc-500 text-sm">
                    "Ask for suggestions, brainstorm ideas, or get help with any part of your campaign."
                </p>
            </div>
        </div>
    }
}

/// Main conversation panel component
#[component]
pub fn ConversationPanel() -> impl IntoView {
    // Local state
    let messages: RwSignal<Vec<ChatMessage>> = RwSignal::new(Vec::new());
    let is_sending = RwSignal::new(false);

    // Handle sending message
    let send_message = Callback::new(move |content: String| {
        // Add user message
        let user_msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            is_streaming: false,
            suggestions: vec![],
            citations: vec![],
        };

        messages.update(|m| m.push(user_msg));
        is_sending.set(true);

        // TODO: Integrate with actual conversation backend
        // For now, simulate a response
        spawn_local(async move {
            // Simulate delay
            gloo_timers::future::TimeoutFuture::new(1000).await;

            let assistant_msg = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content: format!(
                    "I'd be happy to help with that! Based on your current wizard configuration, here are some suggestions for \"{}\".\n\n\
                    Would you like me to elaborate on any of these points?",
                    content
                ),
                timestamp: chrono::Utc::now().to_rfc3339(),
                is_streaming: false,
                suggestions: vec![
                    Suggestion {
                        id: "1".to_string(),
                        text: "Apply suggestion".to_string(),
                        suggestion_type: SuggestionType::Accept,
                        data: None,
                    },
                    Suggestion {
                        id: "2".to_string(),
                        text: "Tell me more".to_string(),
                        suggestion_type: SuggestionType::Expand,
                        data: None,
                    },
                ],
                citations: vec![],
            };

            messages.update(|m| m.push(assistant_msg));
            is_sending.set(false);
        });
    });

    view! {
        <div class="flex flex-col h-full">
            // Header with context
            <ContextHeader />

            // Messages area
            <div class="flex-1 overflow-y-auto p-4">
                {move || {
                    let msgs = messages.get();
                    if msgs.is_empty() {
                        view! { <EmptyState /> }.into_any()
                    } else {
                        msgs.iter().map(|m| view! {
                            <MessageBubble message=m.clone() />
                        }).collect_view().into_any()
                    }
                }}

                // Loading indicator
                <Show when=move || is_sending.get()>
                    <div class="flex justify-start mb-4">
                        <div class="bg-zinc-800 rounded-lg px-4 py-3">
                            <div class="flex items-center gap-2 text-zinc-400">
                                <div class="flex gap-1">
                                    <span class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 0ms" />
                                    <span class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 150ms" />
                                    <span class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 300ms" />
                                </div>
                                <span class="text-sm">"Thinking..."</span>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>

            // Input area
            <MessageInput on_send=send_message is_sending=Signal::derive(move || is_sending.get()) />
        </div>
    }
}
