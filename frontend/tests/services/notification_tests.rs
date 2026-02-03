//! Notification Service Tests
//!
//! Tests for NotificationService toast lifecycle, toast types, and actions.

use leptos::prelude::*;
use std::sync::Arc;
use ttrpg_assistant_frontend::services::notification_service::{
    provide_notification_state, Notification, NotificationState, ToastAction, ToastType,
};
use uuid::Uuid;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// ToastType Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toast_type_equality() {
    assert_eq!(ToastType::Success, ToastType::Success);
    assert_eq!(ToastType::Error, ToastType::Error);
    assert_eq!(ToastType::Warning, ToastType::Warning);
    assert_eq!(ToastType::Info, ToastType::Info);

    assert_ne!(ToastType::Success, ToastType::Error);
    assert_ne!(ToastType::Warning, ToastType::Info);
}

#[wasm_bindgen_test]
fn test_toast_type_clone() {
    let original = ToastType::Success;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

// ============================================================================
// ToastAction Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toast_action_creation() {
    let action = ToastAction {
        label: "Retry".to_string(),
        handler: Arc::new(|| {
            // Handler logic
        }),
    };

    assert_eq!(action.label, "Retry");
}

#[wasm_bindgen_test]
fn test_toast_action_debug() {
    let action = ToastAction {
        label: "Test Action".to_string(),
        handler: Arc::new(|| {}),
    };

    let debug_str = format!("{:?}", action);
    assert!(debug_str.contains("ToastAction"));
    assert!(debug_str.contains("Test Action"));
}

#[wasm_bindgen_test]
fn test_toast_action_equality_same_handler() {
    let handler: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});

    let action1 = ToastAction {
        label: "Same".to_string(),
        handler: handler.clone(),
    };

    let action2 = ToastAction {
        label: "Same".to_string(),
        handler: handler.clone(),
    };

    // Same Arc pointer should be equal
    assert_eq!(action1, action2);
}

#[wasm_bindgen_test]
fn test_toast_action_equality_different_handler() {
    let action1 = ToastAction {
        label: "Action".to_string(),
        handler: Arc::new(|| {}),
    };

    let action2 = ToastAction {
        label: "Action".to_string(),
        handler: Arc::new(|| {}),
    };

    // Different Arc pointers should not be equal (even with same label)
    assert_ne!(action1, action2);
}

#[wasm_bindgen_test]
fn test_toast_action_equality_different_label() {
    let handler: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});

    let action1 = ToastAction {
        label: "First".to_string(),
        handler: handler.clone(),
    };

    let action2 = ToastAction {
        label: "Second".to_string(),
        handler: handler.clone(),
    };

    // Different labels should not be equal
    assert_ne!(action1, action2);
}

// ============================================================================
// Notification Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_notification_creation() {
    let notification = Notification {
        id: Uuid::new_v4(),
        toast_type: ToastType::Success,
        title: "Success!".to_string(),
        message: Some("Operation completed".to_string()),
        action: None,
        duration_ms: Some(5000),
    };

    assert_eq!(notification.title, "Success!");
    assert_eq!(notification.toast_type, ToastType::Success);
    assert!(notification.message.is_some());
    assert!(notification.action.is_none());
    assert_eq!(notification.duration_ms, Some(5000));
}

#[wasm_bindgen_test]
fn test_notification_without_message() {
    let notification = Notification {
        id: Uuid::new_v4(),
        toast_type: ToastType::Info,
        title: "Info".to_string(),
        message: None,
        action: None,
        duration_ms: Some(3000),
    };

    assert!(notification.message.is_none());
}

#[wasm_bindgen_test]
fn test_notification_with_action() {
    let action = ToastAction {
        label: "Undo".to_string(),
        handler: Arc::new(|| {}),
    };

    let notification = Notification {
        id: Uuid::new_v4(),
        toast_type: ToastType::Warning,
        title: "Item deleted".to_string(),
        message: Some("Click undo to restore".to_string()),
        action: Some(action),
        duration_ms: None, // No auto-dismiss when action present
    };

    assert!(notification.action.is_some());
    assert_eq!(notification.action.as_ref().unwrap().label, "Undo");
    assert!(notification.duration_ms.is_none());
}

#[wasm_bindgen_test]
fn test_notification_equality() {
    let id = Uuid::new_v4();

    let notification1 = Notification {
        id,
        toast_type: ToastType::Success,
        title: "Test".to_string(),
        message: None,
        action: None,
        duration_ms: Some(5000),
    };

    let notification2 = Notification {
        id,
        toast_type: ToastType::Success,
        title: "Test".to_string(),
        message: None,
        action: None,
        duration_ms: Some(5000),
    };

    assert_eq!(notification1, notification2);
}

// ============================================================================
// NotificationState Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_notification_state_new() {
    let state = NotificationState::new();
    assert!(state.notifications.get().is_empty());
}

#[wasm_bindgen_test]
fn test_notification_state_add() {
    let state = NotificationState::new();

    // Add a notification
    state.add(
        ToastType::Success,
        "Success".to_string(),
        Some("Message".to_string()),
        None,
    );

    let notifications = state.notifications.get();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].title, "Success");
    assert_eq!(notifications[0].toast_type, ToastType::Success);
}

#[wasm_bindgen_test]
fn test_notification_state_add_multiple() {
    let state = NotificationState::new();

    // Add multiple notifications
    state.add(ToastType::Success, "First".to_string(), None, None);
    state.add(ToastType::Info, "Second".to_string(), None, None);
    state.add(ToastType::Warning, "Third".to_string(), None, None);

    let notifications = state.notifications.get();
    assert_eq!(notifications.len(), 3);
    assert_eq!(notifications[0].title, "First");
    assert_eq!(notifications[1].title, "Second");
    assert_eq!(notifications[2].title, "Third");
}

#[wasm_bindgen_test]
fn test_notification_state_remove() {
    let state = NotificationState::new();

    // Add a notification
    state.add(ToastType::Success, "To Remove".to_string(), None, None);
    let notifications = state.notifications.get();
    let id = notifications[0].id;

    // Remove the notification
    state.remove(id);

    assert!(state.notifications.get().is_empty());
}

#[wasm_bindgen_test]
fn test_notification_state_remove_specific() {
    let state = NotificationState::new();

    // Add multiple notifications
    state.add(ToastType::Success, "Keep 1".to_string(), None, None);
    state.add(ToastType::Info, "Remove Me".to_string(), None, None);
    state.add(ToastType::Warning, "Keep 2".to_string(), None, None);

    let notifications = state.notifications.get();
    let id_to_remove = notifications[1].id;

    // Remove middle notification
    state.remove(id_to_remove);

    let remaining = state.notifications.get();
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].title, "Keep 1");
    assert_eq!(remaining[1].title, "Keep 2");
}

#[wasm_bindgen_test]
fn test_notification_state_remove_nonexistent() {
    let state = NotificationState::new();

    // Add a notification
    state.add(ToastType::Success, "Test".to_string(), None, None);

    // Try to remove a non-existent ID
    let fake_id = Uuid::new_v4();
    state.remove(fake_id);

    // Original notification should still be there
    assert_eq!(state.notifications.get().len(), 1);
}

// ============================================================================
// Toast Duration Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toast_duration_auto_dismiss() {
    let state = NotificationState::new();

    // Success toast without action should have duration
    state.add(ToastType::Success, "Auto dismiss".to_string(), None, None);

    let notifications = state.notifications.get();
    assert!(notifications[0].duration_ms.is_some());
    assert_eq!(notifications[0].duration_ms, Some(5000));
}

#[wasm_bindgen_test]
fn test_toast_duration_info_type() {
    let state = NotificationState::new();

    // Info toast without action should have duration
    state.add(ToastType::Info, "Info toast".to_string(), None, None);

    let notifications = state.notifications.get();
    assert!(notifications[0].duration_ms.is_some());
}

#[wasm_bindgen_test]
fn test_toast_duration_warning_type() {
    let state = NotificationState::new();

    // Warning toast without action should have duration
    state.add(ToastType::Warning, "Warning toast".to_string(), None, None);

    let notifications = state.notifications.get();
    assert!(notifications[0].duration_ms.is_some());
}

#[wasm_bindgen_test]
fn test_toast_duration_error_no_auto_dismiss() {
    let state = NotificationState::new();

    // Error toast should NOT auto-dismiss (no duration)
    state.add(ToastType::Error, "Error toast".to_string(), None, None);

    let notifications = state.notifications.get();
    assert!(notifications[0].duration_ms.is_none());
}

#[wasm_bindgen_test]
fn test_toast_duration_with_action_no_auto_dismiss() {
    let state = NotificationState::new();

    let action = ToastAction {
        label: "Action".to_string(),
        handler: Arc::new(|| {}),
    };

    // Toast with action should NOT auto-dismiss
    state.add(
        ToastType::Success,
        "With action".to_string(),
        None,
        Some(action),
    );

    let notifications = state.notifications.get();
    assert!(notifications[0].duration_ms.is_none());
}

// ============================================================================
// Toast Lifecycle Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toast_lifecycle_create_display_dismiss() {
    let state = NotificationState::new();

    // Create
    state.add(ToastType::Success, "Lifecycle Test".to_string(), None, None);
    assert_eq!(state.notifications.get().len(), 1);

    // Get ID for dismissal
    let id = state.notifications.get()[0].id;

    // Display (notifications are immediately visible)
    let notification = &state.notifications.get()[0];
    assert_eq!(notification.title, "Lifecycle Test");

    // Dismiss
    state.remove(id);
    assert!(state.notifications.get().is_empty());
}

#[wasm_bindgen_test]
fn test_toast_queue_fifo() {
    let state = NotificationState::new();

    // Add in order
    state.add(ToastType::Info, "First".to_string(), None, None);
    state.add(ToastType::Info, "Second".to_string(), None, None);
    state.add(ToastType::Info, "Third".to_string(), None, None);

    // Should be in FIFO order (first added = first in list)
    let notifications = state.notifications.get();
    assert_eq!(notifications[0].title, "First");
    assert_eq!(notifications[1].title, "Second");
    assert_eq!(notifications[2].title, "Third");
}

// ============================================================================
// Context Provider Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_provide_notification_state_mounts() {
    // Test that provide_notification_state can be called without panicking
    leptos::mount::mount_to_body(|| {
        provide_notification_state();

        view! {
            <div id="notification-test">"Notification state provided"</div>
        }
    });
}

// ============================================================================
// Toast Type Variants Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_all_toast_types_can_be_added() {
    let state = NotificationState::new();

    // Add all toast types
    state.add(ToastType::Success, "Success".to_string(), None, None);
    state.add(ToastType::Error, "Error".to_string(), None, None);
    state.add(ToastType::Warning, "Warning".to_string(), None, None);
    state.add(ToastType::Info, "Info".to_string(), None, None);

    let notifications = state.notifications.get();
    assert_eq!(notifications.len(), 4);

    // Verify types
    assert_eq!(notifications[0].toast_type, ToastType::Success);
    assert_eq!(notifications[1].toast_type, ToastType::Error);
    assert_eq!(notifications[2].toast_type, ToastType::Warning);
    assert_eq!(notifications[3].toast_type, ToastType::Info);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_empty_title() {
    let state = NotificationState::new();

    // Empty title should still work
    state.add(ToastType::Info, "".to_string(), None, None);

    let notifications = state.notifications.get();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].title, "");
}

#[wasm_bindgen_test]
fn test_long_message() {
    let state = NotificationState::new();

    let long_message = "A".repeat(1000);
    state.add(
        ToastType::Info,
        "Long Message".to_string(),
        Some(long_message.clone()),
        None,
    );

    let notifications = state.notifications.get();
    assert_eq!(notifications[0].message, Some(long_message));
}

#[wasm_bindgen_test]
fn test_special_characters_in_title() {
    let state = NotificationState::new();

    let special_title = "<script>alert('xss')</script> & \"quotes\" 'apostrophe'";
    state.add(ToastType::Info, special_title.to_string(), None, None);

    let notifications = state.notifications.get();
    assert_eq!(notifications[0].title, special_title);
}

#[wasm_bindgen_test]
fn test_unicode_in_message() {
    let state = NotificationState::new();

    let unicode_message = "Save completed successfully";
    state.add(
        ToastType::Success,
        "Success".to_string(),
        Some(unicode_message.to_string()),
        None,
    );

    let notifications = state.notifications.get();
    assert_eq!(notifications[0].message, Some(unicode_message.to_string()));
}

#[wasm_bindgen_test]
fn test_rapid_add_remove() {
    let state = NotificationState::new();

    // Rapidly add and remove
    for i in 0..10 {
        state.add(ToastType::Info, format!("Toast {}", i), None, None);
    }

    assert_eq!(state.notifications.get().len(), 10);

    // Remove all
    let ids: Vec<Uuid> = state.notifications.get().iter().map(|n| n.id).collect();
    for id in ids {
        state.remove(id);
    }

    assert!(state.notifications.get().is_empty());
}
