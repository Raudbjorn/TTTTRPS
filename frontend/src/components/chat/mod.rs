pub mod chat_message;
pub mod personality_selector;

pub use chat_message::ChatMessage;
pub use personality_selector::{PersonalityIndicator, PersonalitySelector};

use crate::components::design_system::{Button, ButtonVariant, Input};
use crate::services::chat_session_service::use_chat_session_service;
use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;

/// Main Chat component - the primary DM interface with streaming support
#[component]
pub fn Chat() -> impl IntoView {
    let chat_service = use_chat_session_service();

    // Bind to service signals
    let message_input = chat_service.input;
    let messages = chat_service.messages;
    let is_loading = chat_service.is_loading;
    let is_loading_history = chat_service.is_loading_history;
    let llm_status = chat_service.llm_status;
    let session_usage = chat_service.session_usage;
    let show_usage_panel = chat_service.show_usage_panel;

    // Click handler for send button
    let on_send_click = move |_: ev::MouseEvent| {
        chat_service.send_message();
    };

    // Click handler for cancel button
    let on_cancel_click = move |_: ev::MouseEvent| {
        chat_service.cancel_current_stream();
    };

    // Keydown handler for Enter key
    let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            chat_service.send_message();
        }
        // Escape key to cancel stream
        if e.key() == "Escape" && is_loading.get() {
            e.prevent_default();
            chat_service.cancel_current_stream();
        }
    });

    view! {
        <div class="flex flex-col h-screen bg-theme-deep text-theme-primary font-sans transition-colors duration-300">
            // Header
            <div class="p-4 bg-theme-surface border-b border-theme-subtle flex justify-between items-center">
                <div class="flex items-center gap-4">
                    <h1 class="text-xl font-bold">"Sidecar DM"</h1>
                    <span class=move || {
                        if llm_status.get().contains("connected") {
                            "text-xs px-2 py-1 bg-green-900 text-green-300 rounded"
                        } else {
                            "text-xs px-2 py-1 bg-yellow-900 text-yellow-300 rounded"
                        }
                    }>{move || llm_status.get()}</span>
                </div>
                <nav class="flex items-center gap-6">
                    // Usage indicator (only show if tokens used with a paid model)
                    {move || {
                        let usage = session_usage.get();
                        let total_tokens = usage.session_input_tokens + usage.session_output_tokens;
                        let cost_display = if usage.session_cost_usd < 0.01 {
                            "<$0.01".to_string()
                        } else {
                            format!("${:.2}", usage.session_cost_usd)
                        };
                        if total_tokens >= 1 && usage.session_cost_usd > 0.0 {
                            Some(
                                view! {
                                    <Button
                                        variant=ButtonVariant::Secondary
                                        class="text-xs py-1"
                                        on_click=move |_: ev::MouseEvent| {
                                            show_usage_panel.update(|v| *v = !*v);
                                        }
                                    >
                                        <span class="text-gray-400">
                                            {total_tokens} " tokens"
                                        </span>
                                        <span class="text-green-400">{cost_display}</span>
                                    </Button>
                                },
                            )
                        } else {
                            None
                        }
                    }}
                    <HeaderLink href="/campaigns" label="Campaigns">
                        <FolderIcon />
                    </HeaderLink>
                    <HeaderLink href="/character" label="Characters">
                        <UsersIcon />
                    </HeaderLink>
                    <HeaderLink href="/library" label="Library">
                        <BookIcon />
                    </HeaderLink>
                    <HeaderLink href="/settings" label="Settings">
                        <SettingsIcon />
                    </HeaderLink>
                </nav>
            </div>

            // Usage Panel (collapsible)
            {move || {
                if show_usage_panel.get() {
                    let usage = session_usage.get();
                    Some(
                        view! {
                            <div class="p-3 bg-gray-800 border-b border-gray-700">
                                <div class="max-w-4xl mx-auto">
                                    <div class="flex justify-between items-center text-sm">
                                        <div class="flex gap-6">
                                            <div>
                                                <span class="text-gray-400">"Input: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_input_tokens}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Output: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_output_tokens}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Requests: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_requests}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Est. Cost: "</span>
                                                <span class="text-green-400 font-mono">
                                                    {format!("${:.4}", usage.session_cost_usd)}
                                                </span>
                                            </div>
                                        </div>
                                        <button
                                            class="text-gray-500 hover:text-white text-xs"
                                            on:click=move |_| show_usage_panel.set(false)
                                        >
                                            "Close"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                    )
                } else {
                    None
                }
            }}

            // Message Area
            <div class="flex-1 p-4 overflow-y-auto space-y-4">
                {move || {
                    if is_loading_history.get() {
                        // Show loading indicator while session and messages load
                        view! {
                            <div class="flex items-center justify-center h-32 text-gray-400">
                                <div class="flex items-center gap-2">
                                    <div class="w-4 h-4 border-2 border-gray-500 border-t-transparent rounded-full animate-spin"></div>
                                    <span>"Loading conversation..."</span>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        let layout = crate::services::layout_service::use_layout_state();
                        view! {
                            <For
                                each=move || messages.get()
                                key=|msg| (msg.id, msg.content.len(), msg.is_streaming)
                                children=move |msg| {
                                    let role = msg.role.clone();
                                    let content = msg.content.clone();
                                    let tokens = msg.tokens;
                                    let is_streaming = msg.is_streaming;
                                    let show_tokens = layout.show_token_usage.get();
                                    let on_play_handler = if role == "assistant" && !is_streaming {
                                        let content_for_play = content.clone();
                                        // Need to clone chat_service for the closure
                                        let service = chat_service;
                                        Some(Callback::new(move |_: ()| service.play_message(content_for_play.clone())))
                                    } else {
                                        None
                                    };
                                    view! {
                                        <ChatMessage
                                            role=role
                                            content=content
                                            tokens=tokens
                                            is_streaming=is_streaming
                                            on_play=on_play_handler
                                            show_tokens=show_tokens
                                        />
                                    }
                                }
                            />
                        }.into_any()
                    }
                }}
            </div>

            // Input Area
            <div class="p-4 bg-theme-surface border-t border-theme-subtle">
                <div class="flex gap-2 max-w-4xl mx-auto">
                    <div class="flex-1">
                        <Input
                            value=message_input
                            placeholder="Ask the DM... (Escape to cancel)"
                            disabled=Signal::derive(move || is_loading.get() || is_loading_history.get())
                            on_keydown=on_keydown
                        />
                    </div>
                    {move || {
                        if is_loading_history.get() {
                            // Session loading - show disabled button with spinner
                            view! {
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=move |_: ev::MouseEvent| {}
                                    disabled=true
                                    class="opacity-50 cursor-not-allowed"
                                >
                                    <div class="w-4 h-4 mr-2 border-2 border-gray-400 border-t-transparent rounded-full animate-spin"></div>
                                    "..."
                                </Button>
                            }.into_any()
                        } else if is_loading.get() {
                            view! {
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=on_cancel_click
                                    class="bg-red-900 hover:bg-red-800 border-red-700"
                                >
                                    <svg class="w-4 h-4 mr-1" viewBox="0 0 24 24" fill="currentColor">
                                        <path d="M6 6h12v12H6z"/>
                                    </svg>
                                    "Stop"
                                </Button>
                            }.into_any()
                        } else {
                            view! {
                                <Button on_click=on_send_click>
                                    "Send"
                                </Button>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
// SVG Icon Components for Header
#[component]
fn FolderIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}

#[component]
fn UsersIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="9" cy="7" r="4"></circle>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
        </svg>
    }
}

#[component]
fn BookIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
        </svg>
    }
}

#[component]
fn SettingsIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
        </svg>
    }
}

// Navigation Link component with tooltip (dynamic text/icon mode)
#[component]
pub fn HeaderLink(href: &'static str, label: &'static str, children: Children) -> impl IntoView {
    let layout_state = crate::services::layout_service::use_layout_state();
    let text_mode = layout_state.text_navigation;
    let icon_children = children();

    view! {
        <A href=href attr:class="group relative text-theme-muted hover:text-theme-primary transition-colors p-2 rounded hover:bg-white/5 flex items-center justify-center">
            // Icon (Hidden in text mode, but rendered once to avoid re-creation issues)
            <div class=move || if text_mode.get() { "hidden" } else { "" }>
                {icon_children}
            </div>

            // Text Label (Dynamic)
            {move || {
                if text_mode.get() {
                    view! { <span class="font-medium text-sm px-2 animate-fade-in">{label}</span> }.into_any()
                } else {
                    view! { <span class="hidden" /> }.into_any()
                }
            }}

            // Tooltip (Only in icon mode)
            <div
                class=move || format!(
                    "absolute top-full mt-2 left-1/2 -translate-x-1/2 bg-[var(--bg-elevated)] text-[var(--text-primary)] text-xs font-medium px-2 py-1 rounded shadow-lg opacity-0 group-hover:opacity-100 group-hover:translate-y-1 group-focus:opacity-100 transition-all duration-200 whitespace-nowrap border border-[var(--border-subtle)] z-[100] pointer-events-none backdrop-blur-md {}",
                    if text_mode.get() { "hidden" } else { "" }
                )
                role="tooltip"
            >
                {label}
            </div>
        </A>
    }
}
