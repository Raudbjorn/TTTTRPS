//! Layout Service Tests
//!
//! Tests for LayoutService responsive states, sidebar/infopanel visibility,
//! width management, and view navigation.

use leptos::prelude::*;
use ttrpg_assistant_frontend::services::layout_service::{
    provide_layout_state, use_layout_state, LayoutState, ViewType,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// ViewType Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_view_type_default() {
    let view = ViewType::default();
    assert_eq!(view, ViewType::Home);
}

#[wasm_bindgen_test]
fn test_view_type_equality() {
    assert_eq!(ViewType::Home, ViewType::Home);
    assert_eq!(ViewType::Campaigns, ViewType::Campaigns);
    assert_eq!(ViewType::Chat, ViewType::Chat);
    assert_eq!(ViewType::Library, ViewType::Library);
    assert_eq!(ViewType::Graph, ViewType::Graph);
    assert_eq!(ViewType::Settings, ViewType::Settings);

    assert_ne!(ViewType::Home, ViewType::Settings);
    assert_ne!(ViewType::Campaigns, ViewType::Chat);
}

#[wasm_bindgen_test]
fn test_view_type_display_name() {
    assert_eq!(ViewType::Home.display_name(), "Home");
    assert_eq!(ViewType::Campaigns.display_name(), "Campaigns");
    assert_eq!(ViewType::Chat.display_name(), "Chat");
    assert_eq!(ViewType::Library.display_name(), "Library");
    assert_eq!(ViewType::Graph.display_name(), "Graph");
    assert_eq!(ViewType::Settings.display_name(), "Settings");
}

#[wasm_bindgen_test]
fn test_view_type_icon_name() {
    assert_eq!(ViewType::Home.icon_name(), "home");
    assert_eq!(ViewType::Campaigns.icon_name(), "folder");
    assert_eq!(ViewType::Chat.icon_name(), "message-circle");
    assert_eq!(ViewType::Library.icon_name(), "book");
    assert_eq!(ViewType::Graph.icon_name(), "git-branch");
    assert_eq!(ViewType::Settings.icon_name(), "settings");
}

// ============================================================================
// LayoutState Creation Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_layout_state_new() {
    let state = LayoutState::new();

    // Check default values
    assert!(state.sidebar_visible.get());
    assert_eq!(state.sidebar_width.get(), 280);
    assert!(state.infopanel_visible.get());
    assert_eq!(state.infopanel_width.get(), 320);
    assert_eq!(state.active_view.get(), ViewType::Home);
    assert!(!state.text_navigation.get());
}

#[wasm_bindgen_test]
fn test_layout_state_default() {
    let state = LayoutState::default();

    // Default should be same as new()
    assert!(state.sidebar_visible.get());
    assert_eq!(state.sidebar_width.get(), 280);
    assert!(state.infopanel_visible.get());
    assert_eq!(state.infopanel_width.get(), 320);
    assert_eq!(state.active_view.get(), ViewType::Home);
    assert!(!state.text_navigation.get());
}

// ============================================================================
// Sidebar Visibility Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toggle_sidebar() {
    let state = LayoutState::new();

    // Initially visible
    assert!(state.sidebar_visible.get());

    // Toggle to hidden
    state.toggle_sidebar();
    assert!(!state.sidebar_visible.get());

    // Toggle back to visible
    state.toggle_sidebar();
    assert!(state.sidebar_visible.get());
}

#[wasm_bindgen_test]
fn test_set_sidebar_visible() {
    let state = LayoutState::new();

    // Set to hidden explicitly
    state.set_sidebar_visible(false);
    assert!(!state.sidebar_visible.get());

    // Set to visible explicitly
    state.set_sidebar_visible(true);
    assert!(state.sidebar_visible.get());

    // Setting same value should work
    state.set_sidebar_visible(true);
    assert!(state.sidebar_visible.get());
}

// ============================================================================
// Infopanel Visibility Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_toggle_infopanel() {
    let state = LayoutState::new();

    // Initially visible
    assert!(state.infopanel_visible.get());

    // Toggle to hidden
    state.toggle_infopanel();
    assert!(!state.infopanel_visible.get());

    // Toggle back to visible
    state.toggle_infopanel();
    assert!(state.infopanel_visible.get());
}

#[wasm_bindgen_test]
fn test_set_infopanel_visible() {
    let state = LayoutState::new();

    // Set to hidden explicitly
    state.set_infopanel_visible(false);
    assert!(!state.infopanel_visible.get());

    // Set to visible explicitly
    state.set_infopanel_visible(true);
    assert!(state.infopanel_visible.get());
}

// ============================================================================
// Sidebar Width Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_set_sidebar_width_normal() {
    let state = LayoutState::new();

    // Set to a normal value
    state.set_sidebar_width(300);
    assert_eq!(state.sidebar_width.get(), 300);

    // Set to another normal value
    state.set_sidebar_width(350);
    assert_eq!(state.sidebar_width.get(), 350);
}

#[wasm_bindgen_test]
fn test_set_sidebar_width_minimum_clamp() {
    let state = LayoutState::new();

    // Set below minimum (200px)
    state.set_sidebar_width(100);
    assert_eq!(state.sidebar_width.get(), 200); // Clamped to 200

    // Set to exactly minimum
    state.set_sidebar_width(200);
    assert_eq!(state.sidebar_width.get(), 200);

    // Set to negative (edge case)
    state.set_sidebar_width(-50);
    assert_eq!(state.sidebar_width.get(), 200); // Clamped to 200
}

#[wasm_bindgen_test]
fn test_set_sidebar_width_maximum_clamp() {
    let state = LayoutState::new();

    // Set above maximum (500px)
    state.set_sidebar_width(600);
    assert_eq!(state.sidebar_width.get(), 500); // Clamped to 500

    // Set to exactly maximum
    state.set_sidebar_width(500);
    assert_eq!(state.sidebar_width.get(), 500);
}

// ============================================================================
// Infopanel Width Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_set_infopanel_width_normal() {
    let state = LayoutState::new();

    // Set to a normal value
    state.set_infopanel_width(400);
    assert_eq!(state.infopanel_width.get(), 400);

    // Set to another normal value
    state.set_infopanel_width(250);
    assert_eq!(state.infopanel_width.get(), 250);
}

#[wasm_bindgen_test]
fn test_set_infopanel_width_minimum_clamp() {
    let state = LayoutState::new();

    // Set below minimum (200px)
    state.set_infopanel_width(100);
    assert_eq!(state.infopanel_width.get(), 200); // Clamped to 200

    // Set to exactly minimum
    state.set_infopanel_width(200);
    assert_eq!(state.infopanel_width.get(), 200);
}

#[wasm_bindgen_test]
fn test_set_infopanel_width_maximum_clamp() {
    let state = LayoutState::new();

    // Set above maximum (600px)
    state.set_infopanel_width(700);
    assert_eq!(state.infopanel_width.get(), 600); // Clamped to 600

    // Set to exactly maximum
    state.set_infopanel_width(600);
    assert_eq!(state.infopanel_width.get(), 600);
}

// ============================================================================
// View Navigation Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_navigate_to() {
    let state = LayoutState::new();

    // Initially at Home
    assert_eq!(state.active_view.get(), ViewType::Home);

    // Navigate to Campaigns
    state.navigate_to(ViewType::Campaigns);
    assert_eq!(state.active_view.get(), ViewType::Campaigns);

    // Navigate to Settings
    state.navigate_to(ViewType::Settings);
    assert_eq!(state.active_view.get(), ViewType::Settings);

    // Navigate to Chat
    state.navigate_to(ViewType::Chat);
    assert_eq!(state.active_view.get(), ViewType::Chat);

    // Navigate back to Home
    state.navigate_to(ViewType::Home);
    assert_eq!(state.active_view.get(), ViewType::Home);
}

#[wasm_bindgen_test]
fn test_is_active() {
    let state = LayoutState::new();

    // Initially Home is active
    assert!(state.is_active(ViewType::Home));
    assert!(!state.is_active(ViewType::Campaigns));
    assert!(!state.is_active(ViewType::Settings));

    // Navigate to Campaigns
    state.navigate_to(ViewType::Campaigns);
    assert!(!state.is_active(ViewType::Home));
    assert!(state.is_active(ViewType::Campaigns));
    assert!(!state.is_active(ViewType::Settings));

    // Navigate to Settings
    state.navigate_to(ViewType::Settings);
    assert!(!state.is_active(ViewType::Home));
    assert!(!state.is_active(ViewType::Campaigns));
    assert!(state.is_active(ViewType::Settings));
}

// ============================================================================
// Text Navigation Toggle Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_text_navigation_toggle() {
    let state = LayoutState::new();

    // Initially icons (false)
    assert!(!state.text_navigation.get());

    // Toggle to text
    state.text_navigation.set(true);
    assert!(state.text_navigation.get());

    // Toggle back to icons
    state.text_navigation.set(false);
    assert!(!state.text_navigation.get());
}

// ============================================================================
// Responsive State Simulation Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_mobile_layout_simulation() {
    // Simulate mobile layout by hiding sidebar and infopanel
    let state = LayoutState::new();

    // Simulate mobile breakpoint - hide both panels
    state.set_sidebar_visible(false);
    state.set_infopanel_visible(false);

    assert!(!state.sidebar_visible.get());
    assert!(!state.infopanel_visible.get());
}

#[wasm_bindgen_test]
fn test_tablet_layout_simulation() {
    // Simulate tablet layout - sidebar visible, no infopanel
    let state = LayoutState::new();

    // Tablet breakpoint
    state.set_sidebar_visible(true);
    state.set_infopanel_visible(false);
    state.set_sidebar_width(250); // Slightly narrower

    assert!(state.sidebar_visible.get());
    assert!(!state.infopanel_visible.get());
    assert_eq!(state.sidebar_width.get(), 250);
}

#[wasm_bindgen_test]
fn test_desktop_layout_simulation() {
    // Simulate desktop layout - both panels visible
    let state = LayoutState::new();

    // Desktop breakpoint - full width panels
    state.set_sidebar_visible(true);
    state.set_infopanel_visible(true);
    state.set_sidebar_width(280);
    state.set_infopanel_width(320);

    assert!(state.sidebar_visible.get());
    assert!(state.infopanel_visible.get());
    assert_eq!(state.sidebar_width.get(), 280);
    assert_eq!(state.infopanel_width.get(), 320);
}

#[wasm_bindgen_test]
fn test_wide_desktop_layout_simulation() {
    // Simulate wide desktop layout - maximum panel widths
    let state = LayoutState::new();

    // Wide desktop breakpoint
    state.set_sidebar_visible(true);
    state.set_infopanel_visible(true);
    state.set_sidebar_width(500); // Max sidebar
    state.set_infopanel_width(600); // Max infopanel

    assert!(state.sidebar_visible.get());
    assert!(state.infopanel_visible.get());
    assert_eq!(state.sidebar_width.get(), 500);
    assert_eq!(state.infopanel_width.get(), 600);
}

// ============================================================================
// Layout State Reactivity Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_layout_signals_are_reactive() {
    let state = LayoutState::new();

    // Test that signals update immediately
    let initial_sidebar = state.sidebar_visible.get();
    state.toggle_sidebar();
    let updated_sidebar = state.sidebar_visible.get();

    assert_ne!(initial_sidebar, updated_sidebar);
}

#[wasm_bindgen_test]
fn test_multiple_state_changes() {
    let state = LayoutState::new();

    // Perform multiple state changes
    state.navigate_to(ViewType::Campaigns);
    state.toggle_sidebar();
    state.set_infopanel_width(400);
    state.text_navigation.set(true);

    // Verify all changes persisted
    assert_eq!(state.active_view.get(), ViewType::Campaigns);
    assert!(!state.sidebar_visible.get());
    assert_eq!(state.infopanel_width.get(), 400);
    assert!(state.text_navigation.get());
}

// ============================================================================
// Context Provider Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_provide_layout_state_mounts() {
    // Test that provide_layout_state can be called without panicking
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <div id="layout-test">"Layout state provided"</div>
        }
    });
}

#[wasm_bindgen_test]
fn test_use_layout_state_access() {
    // Test that use_layout_state can access the provided state
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        // Access layout state
        let layout = use_layout_state();

        // Verify we can read from it
        let _view = layout.active_view.get();
        let _sidebar = layout.sidebar_visible.get();

        view! {
            <div id="layout-access-test">"Layout state accessed"</div>
        }
    });
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_rapid_toggle() {
    let state = LayoutState::new();

    // Rapidly toggle sidebar
    for _ in 0..10 {
        state.toggle_sidebar();
    }

    // After even number of toggles, should be back to initial (visible)
    assert!(state.sidebar_visible.get());

    // One more toggle
    state.toggle_sidebar();
    assert!(!state.sidebar_visible.get());
}

#[wasm_bindgen_test]
fn test_boundary_width_values() {
    let state = LayoutState::new();

    // Test exact boundaries for sidebar
    state.set_sidebar_width(200); // Exact min
    assert_eq!(state.sidebar_width.get(), 200);

    state.set_sidebar_width(500); // Exact max
    assert_eq!(state.sidebar_width.get(), 500);

    state.set_sidebar_width(199); // Just below min
    assert_eq!(state.sidebar_width.get(), 200);

    state.set_sidebar_width(501); // Just above max
    assert_eq!(state.sidebar_width.get(), 500);
}
