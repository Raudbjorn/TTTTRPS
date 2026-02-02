use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{
    cancel_stream, check_llm_health, get_session_usage, listen_chat_chunks_async, stream_chat,
    ChatChunk, SessionUsage, StreamingChatMessage,
    get_or_create_chat_session, get_chat_messages, add_chat_message, update_chat_message,
    link_chat_to_game_session, speak,
};
use crate::services::notification_service::show_error;
use crate::services::chat_context::try_use_chat_context;

/// Message in the chat history
#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub id: usize,
    pub role: String,
    pub content: String,
    pub tokens: Option<(u32, u32)>,
    pub is_streaming: bool,
    pub stream_id: Option<String>,
    pub persistent_id: Option<String>,
}

const WELCOME_MESSAGE: &str = "Welcome to Sidecar DM! I'm your AI-powered TTRPG assistant. Configure an LLM provider in Settings to get started.";

fn create_welcome_message() -> Message {
    Message {
        id: 0,
        role: "assistant".to_string(),
        content: WELCOME_MESSAGE.to_string(),
        tokens: None,
        is_streaming: false,
        stream_id: None,
        persistent_id: None,
    }
}

#[derive(Clone, Copy)]
pub struct ChatSessionService {
    pub messages: RwSignal<Vec<Message>>,
    pub input: RwSignal<String>,
    pub is_loading: RwSignal<bool>,
    pub is_loading_history: RwSignal<bool>,
    pub session_id: RwSignal<Option<String>>,
    pub session_usage: RwSignal<SessionUsage>,
    pub llm_status: RwSignal<String>,
    pub show_usage_panel: RwSignal<bool>,
    pub next_message_id: RwSignal<usize>,
    pub current_stream_id: RwSignal<Option<String>>,
    pub streaming_message_id: RwSignal<Option<usize>>,
    pub streaming_persistent_id: RwSignal<Option<String>>,
}

impl ChatSessionService {
    pub fn new() -> Self {
        let service = Self {
            messages: RwSignal::new(Vec::new()),
            input: RwSignal::new(String::new()),
            is_loading: RwSignal::new(false),
            is_loading_history: RwSignal::new(true),
            session_id: RwSignal::new(None),
            session_usage: RwSignal::new(SessionUsage {
                session_input_tokens: 0,
                session_output_tokens: 0,
                session_requests: 0,
                session_cost_usd: 0.0,
            }),
            llm_status: RwSignal::new("Checking...".to_string()),
            show_usage_panel: RwSignal::new(false),
            next_message_id: RwSignal::new(1),
            current_stream_id: RwSignal::new(None),
            streaming_message_id: RwSignal::new(None),
            streaming_persistent_id: RwSignal::new(None),
        };

        service.init();
        service
    }

    fn init(&self) {
        let service = *self;
        
        // 1. Load History
        spawn_local(async move {
            match get_or_create_chat_session().await {
                Ok(session) => {
                    service.session_id.set(Some(session.id.clone()));

                    match get_chat_messages(session.id, Some(100)).await {
                        Ok(stored_messages) => {
                            if stored_messages.is_empty() {
                                service.messages.set(vec![create_welcome_message()]);
                                service.next_message_id.set(1);
                            } else {
                                let ui_messages: Vec<Message> = stored_messages
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, m)| Message {
                                        id: idx,
                                        role: m.role.clone(),
                                        content: m.content.clone(),
                                        tokens: match (m.tokens_input, m.tokens_output) {
                                            (Some(i), Some(o)) => Some((i as u32, o as u32)),
                                            _ => None,
                                        },
                                        is_streaming: m.is_streaming,
                                        stream_id: None,
                                        persistent_id: Some(m.id.clone()),
                                    })
                                    .collect();
                                service.next_message_id.set(ui_messages.len());
                                service.messages.set(ui_messages);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to load chat messages: {}", e);
                            service.messages.set(vec![create_welcome_message()]);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get/create chat session: {}", e);
                    service.messages.set(vec![create_welcome_message()]);
                }
            }
            service.is_loading_history.set(false);
        });

        // 2. Initialize Stream Listener
        spawn_local(async move {
            let _unlisten = listen_chat_chunks_async(move |chunk: ChatChunk| {
                let result = service.messages.try_update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| {
                        m.stream_id.as_ref() == Some(&chunk.stream_id) && m.is_streaming
                    }) {
                        if !chunk.content.is_empty() {
                            msg.content.push_str(&chunk.content);
                        }
                        if chunk.is_final {
                            msg.is_streaming = false;
                            msg.stream_id = None;
                            if chunk.finish_reason.as_deref() == Some("error") {
                                msg.role = "error".to_string();
                            }
                            if let Some(usage) = &chunk.usage {
                                msg.tokens = Some((usage.input_tokens, usage.output_tokens));
                            }
                        }
                        true
                    } else {
                        false
                    }
                });

                match result {
                    Some(true) => {
                        if chunk.is_final {
                            let _ = service.is_loading.try_set(false);
                            let _ = service.current_stream_id.try_set(None);
                            let _ = service.streaming_message_id.try_set(None);

                            if let Some(pid) = service.streaming_persistent_id.get_untracked() {
                                let final_content = service.messages.try_with(|msgs| {
                                    msgs.iter()
                                        .find(|m| m.persistent_id.as_ref() == Some(&pid))
                                        .map(|m| m.content.clone())
                                });

                                if let Some(Some(content)) = final_content {
                                    let tokens = chunk.usage.as_ref().map(|u| (u.input_tokens as i32, u.output_tokens as i32));
                                    let pid_clone = pid.clone();
                                    spawn_local(async move {
                                        if let Err(e) = update_chat_message(pid_clone, content, tokens, false).await {
                                            log::error!("Failed to persist final message: {}", e);
                                        }
                                    });
                                }
                                let _ = service.streaming_persistent_id.try_set(None);
                            }

                            spawn_local(async move {
                                if let Ok(usage) = get_session_usage().await {
                                    let _ = service.session_usage.try_set(usage);
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }).await;
        });

        // 3. Health Check
        spawn_local(async move {
            service.llm_status.set("Checking...".to_string());
            match check_llm_health().await {
                Ok(status) => {
                    if status.healthy {
                        service.llm_status.set(format!("{} connected", status.provider));
                    } else {
                        service.llm_status.set(format!("{}: {}", status.provider, status.message));
                    }
                }
                Err(e) => {
                    service.llm_status.set(format!("Error: {}", e));
                }
            }
        });

        // 4. Link Campaign Effect
        Effect::new(move |_| {
            let session_id = service.session_id.get();
            let campaign_ctx = try_use_chat_context();

            if let (Some(sid), Some(ctx)) = (session_id, campaign_ctx) {
                if let Some(campaign_id) = ctx.campaign_id() {
                    spawn_local(async move {
                        if let Err(e) = link_chat_to_game_session(
                            sid,
                            String::new(),
                            Some(campaign_id),
                        ).await {
                            log::warn!("Failed to link chat session to campaign: {}", e);
                        }
                    });
                }
            }
        });
    }

    pub fn send_message(&self) {
        let msg = self.input.get();
        if msg.trim().is_empty() || self.is_loading.get() {
            return;
        }

        let session_id = match self.session_id.get() {
            Some(id) => id,
            None => {
                show_error("Chat Not Ready", Some("Please wait for the conversation to load."), None);
                return;
            }
        };

        // User Message
        let user_msg_id = self.next_message_id.get();
        self.next_message_id.set(user_msg_id + 1);
        self.messages.update(|msgs| {
            msgs.push(Message {
                id: user_msg_id,
                role: "user".to_string(),
                content: msg.clone(),
                tokens: None,
                is_streaming: false,
                stream_id: None,
                persistent_id: None,
            });
        });

        {
            let sid = session_id.clone();
            let msg_content = msg.clone();
            spawn_local(async move {
                if let Err(e) = add_chat_message(sid, "user".to_string(), msg_content, None).await {
                    log::error!("Failed to persist user message: {}", e);
                }
            });
        }

        // Assistant Placeholder
        let assistant_msg_id = self.next_message_id.get();
        self.next_message_id.set(assistant_msg_id + 1);
        self.messages.update(|msgs| {
            msgs.push(Message {
                id: assistant_msg_id,
                role: "assistant".to_string(),
                content: String::new(),
                tokens: None,
                is_streaming: true,
                stream_id: None,
                persistent_id: None,
            });
        });

        {
            let sid = session_id;
            let streaming_persistent_id = self.streaming_persistent_id;
            let messages = self.messages;
            spawn_local(async move {
                match add_chat_message(sid, "assistant".to_string(), String::new(), None).await {
                    Ok(record) => {
                        let pid = record.id.clone();
                        streaming_persistent_id.set(Some(record.id));
                        messages.update(|msgs| {
                            if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                                msg.persistent_id = Some(pid);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to persist assistant placeholder: {}", e);
                    }
                }
            });
        }

        self.input.set(String::new());
        self.is_loading.set(true);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let stream_id_clone = stream_id.clone();

        self.current_stream_id.set(Some(stream_id.clone()));
        self.streaming_message_id.set(Some(assistant_msg_id));

        self.messages.update(|msgs| {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                msg.stream_id = Some(stream_id.clone());
            }
        });

        let history: Vec<StreamingChatMessage> = self.messages.get().iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .filter(|m| m.id != assistant_msg_id)
            .map(|m| StreamingChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let system_prompt = {
            let base_prompt = "You are a TTRPG assistant helping a Game Master run engaging tabletop sessions. 
                You have expertise in narrative design, encounter balancing, improvisation, and player engagement. 
                Be helpful, creative, and supportive of the GM's vision.";

            if let Some(chat_ctx) = try_use_chat_context() {
                if let Some(augmentation) = chat_ctx.build_prompt_augmentation() {
                    Some(format!("{}{}", base_prompt, augmentation))
                } else {
                    Some(base_prompt.to_string())
                }
            } else {
                Some(base_prompt.to_string())
            }
        };

        let service = *self;
        spawn_local(async move {
            match stream_chat(history, system_prompt, None, None, Some(stream_id_clone)).await {
                Ok(_) => {}
                Err(e) => {
                    service.messages.update(|msgs| {
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                            msg.role = "error".to_string();
                            msg.content = format!("Streaming error: {}

Course of Action: Check your network connection or verify the LLM provider settings.", e);
                            msg.is_streaming = false;
                        }
                    });
                    service.is_loading.set(false);
                    service.streaming_message_id.set(None);
                    service.current_stream_id.set(None);
                    show_error("Streaming Failed", Some(&e), None);
                }
            }
        });
    }

    pub fn cancel_current_stream(&self) {
        if let Some(stream_id) = self.current_stream_id.get() {
            let stream_id_clone = stream_id.clone();
            spawn_local(async move {
                let _ = cancel_stream(stream_id_clone).await;
            });

            if let Some(msg_id) = self.streaming_message_id.get() {
                self.messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                        msg.is_streaming = false;
                        msg.stream_id = None;
                        if msg.content.is_empty() {
                            msg.content = "[Response canceled]".to_string();
                        } else {
                            msg.content.push_str("

[Stream canceled]");
                        }
                    }
                });
            }

            self.is_loading.set(false);
            self.current_stream_id.set(None);
            self.streaming_message_id.set(None);
        }
    }

    pub fn play_message(&self, text: String) {
        let messages = self.messages;
        let next_id = self.next_message_id;
        
        spawn_local(async move {
            match speak(text).await {
                Ok(Some(result)) => {
                    let mime_type = if result.format == "mp3" { "audio/mpeg" } else { "audio/wav" };
                    let data_url = format!("data:{};base64,{}", mime_type, result.audio_data);

                    if let Ok(audio) = web_sys::HtmlAudioElement::new_with_src(&data_url) {
                        if let Err(e) = audio.play() {
                            log::error!("Failed to play audio: {:?}", e);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    let id = next_id.get();
                    next_id.set(id + 1);
                    messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: "error".to_string(),
                            content: format!("Voice Error: {}", e),
                            tokens: None,
                            is_streaming: false,
                            stream_id: None,
                            persistent_id: None,
                        });
                    });
                }
            }
        });
    }
}

pub fn provide_chat_session_service() {
    provide_context(ChatSessionService::new());
}

pub fn use_chat_session_service() -> ChatSessionService {
    expect_context::<ChatSessionService>()
}
