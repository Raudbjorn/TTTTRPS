use tokio::sync::mpsc;

use super::events::{AppEvent, Focus, Notification};

/// Central application state (Elm architecture).
pub struct AppState {
    /// Whether the app is still running.
    pub running: bool,
    /// Currently focused top-level view.
    pub focus: Focus,
    /// Active notifications (max 3 visible).
    pub notifications: Vec<Notification>,
    /// Monotonic counter for notification IDs.
    notification_counter: u64,
    /// Whether the help modal is open.
    pub show_help: bool,
    /// Whether the command palette is open.
    pub show_command_palette: bool,
    /// Receiver for backend events.
    pub event_rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Sender for pushing events from within the app.
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl AppState {
    pub fn new(
        event_rx: mpsc::UnboundedReceiver<AppEvent>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        Self {
            running: true,
            focus: Focus::Chat,
            notifications: Vec::new(),
            notification_counter: 0,
            show_help: false,
            show_command_palette: false,
            event_rx,
            event_tx,
        }
    }

    /// Push a notification (dedup by message, max 3).
    pub fn push_notification(&mut self, message: String, level: super::events::NotificationLevel) {
        // Dedup: skip if identical message already present
        if self.notifications.iter().any(|n| n.message == message) {
            return;
        }

        self.notification_counter += 1;
        let notification = Notification {
            id: self.notification_counter,
            message,
            level,
            ttl_ticks: 100, // ~5 seconds at 50ms tick
        };

        self.notifications.push(notification);

        // Keep max 3 visible
        while self.notifications.len() > 3 {
            self.notifications.remove(0);
        }
    }

    /// Tick: decrement notification TTLs and dismiss expired.
    pub fn on_tick(&mut self) {
        for n in &mut self.notifications {
            n.ttl_ticks = n.ttl_ticks.saturating_sub(1);
        }
        self.notifications.retain(|n| n.ttl_ticks > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::events::NotificationLevel;

    fn make_app() -> AppState {
        let (tx, rx) = mpsc::unbounded_channel();
        AppState::new(rx, tx)
    }

    #[test]
    fn test_initial_state() {
        let app = make_app();
        assert!(app.running);
        assert_eq!(app.focus, Focus::Chat);
        assert!(app.notifications.is_empty());
        assert!(!app.show_help);
        assert!(!app.show_command_palette);
    }

    #[test]
    fn test_push_notification_dedup() {
        let mut app = make_app();
        app.push_notification("hello".into(), NotificationLevel::Info);
        app.push_notification("hello".into(), NotificationLevel::Info);
        assert_eq!(app.notifications.len(), 1);
    }

    #[test]
    fn test_push_notification_max_3() {
        let mut app = make_app();
        for i in 0..5 {
            app.push_notification(format!("msg {i}"), NotificationLevel::Info);
        }
        assert_eq!(app.notifications.len(), 3);
    }

    #[test]
    fn test_on_tick_dismisses_expired() {
        let mut app = make_app();
        app.push_notification("test".into(), NotificationLevel::Info);
        app.notifications[0].ttl_ticks = 1;
        app.on_tick();
        assert!(app.notifications.is_empty());
    }
}
