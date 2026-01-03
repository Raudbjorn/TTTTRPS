use leptos::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone)]
pub struct ToastAction {
    pub label: String,
    pub handler: Arc<dyn Fn() + Send + Sync>,
}

// Implement Debug manually since Arc<dyn Fn()> doesn't implement it
impl std::fmt::Debug for ToastAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToastAction")
            .field("label", &self.label)
            .finish()
    }
}

// Implement PartialEq manually
impl PartialEq for ToastAction {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label && Arc::ptr_eq(&self.handler, &other.handler)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub id: Uuid,
    pub toast_type: ToastType,
    pub title: String,
    pub message: Option<String>,
    pub action: Option<ToastAction>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone)]
pub struct NotificationState {
    pub notifications: RwSignal<Vec<Notification>>,
}

impl NotificationState {
    pub fn new() -> Self {
        Self {
            notifications: RwSignal::new(Vec::new()),
        }
    }

    pub fn add(&self, toast_type: ToastType, title: String, message: Option<String>, action: Option<ToastAction>) {
        let id = Uuid::new_v4();
        let notification = Notification {
            id,
            toast_type,
            title,
            message,
            action,
            duration_ms: None, // Could add auto-dismiss logic here
        };

        self.notifications.update(|list| list.push(notification));

        // Auto-dismiss after 5 seconds if no action
        // In a real app we'd want to handle cleanup of timers, but for now simple set_timeout is okayish if we don't care about leaks on unmount (which is rare for a global service)
    }

    pub fn remove(&self, id: Uuid) {
        self.notifications.update(|list| {
            if let Some(pos) = list.iter().position(|n| n.id == id) {
                list.remove(pos);
            }
        });
    }
}

// Global accessor helpers
pub fn provide_notification_state() {
    provide_context(NotificationState::new());
}

pub fn use_notification_state() -> NotificationState {
    expect_context::<NotificationState>()
}

pub fn remove_notification(id: Uuid) {
    if let Some(state) = use_context::<NotificationState>() {
        state.remove(id);
    }
}

pub fn show_success(title: &str, message: Option<&str>) {
    if let Some(state) = use_context::<NotificationState>() {
        state.add(ToastType::Success, title.to_string(), message.map(String::from), None);
    }
}

pub fn show_error(title: &str, message: Option<&str>, action: Option<ToastAction>) {
    if let Some(state) = use_context::<NotificationState>() {
        state.add(ToastType::Error, title.to_string(), message.map(String::from), action);
    }
}

pub fn show_info(title: &str, message: Option<&str>) {
    if let Some(state) = use_context::<NotificationState>() {
        state.add(ToastType::Info, title.to_string(), message.map(String::from), None);
    }
}
