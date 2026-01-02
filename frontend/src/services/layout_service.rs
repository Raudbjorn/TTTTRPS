//! Layout Service for Leptos frontend
//!
//! Manages application layout state including sidebar/infopanel visibility,
//! widths, and active view tracking. Uses Leptos signals and context for
//! reactive state management.

use leptos::prelude::*;

/// The different views available in the application
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ViewType {
    #[default]
    Campaigns,
    Chat,
    Library,
    Graph,
    Settings,
}

impl ViewType {
    /// Returns the display name for this view
    pub fn display_name(&self) -> &'static str {
        match self {
            ViewType::Campaigns => "Campaigns",
            ViewType::Chat => "Chat",
            ViewType::Library => "Library",
            ViewType::Graph => "Graph",
            ViewType::Settings => "Settings",
        }
    }

    /// Returns the icon name for this view (for icon components)
    pub fn icon_name(&self) -> &'static str {
        match self {
            ViewType::Campaigns => "folder",
            ViewType::Chat => "message-circle",
            ViewType::Library => "book",
            ViewType::Graph => "git-branch",
            ViewType::Settings => "settings",
        }
    }
}

/// Layout state container holding all reactive signals for layout management.
///
/// This struct is designed to be provided via Leptos context and accessed
/// throughout the component tree using `expect_context::<LayoutState>()`.
#[derive(Clone, Copy)]
pub struct LayoutState {
    /// Whether the sidebar is currently visible
    pub sidebar_visible: RwSignal<bool>,
    /// Current width of the sidebar in pixels
    pub sidebar_width: RwSignal<i32>,
    /// Whether the info panel is currently visible
    pub infopanel_visible: RwSignal<bool>,
    /// Current width of the info panel in pixels
    pub infopanel_width: RwSignal<i32>,
    /// The currently active view in the application
    pub active_view: RwSignal<ViewType>,
}

impl LayoutState {
    /// Create a new LayoutState with default values
    pub fn new() -> Self {
        Self {
            sidebar_visible: RwSignal::new(true),
            sidebar_width: RwSignal::new(280),
            infopanel_visible: RwSignal::new(true),
            infopanel_width: RwSignal::new(320),
            active_view: RwSignal::new(ViewType::default()),
        }
    }

    /// Toggle the sidebar visibility
    pub fn toggle_sidebar(&self) {
        self.sidebar_visible.update(|v| *v = !*v);
    }

    /// Toggle the info panel visibility
    pub fn toggle_infopanel(&self) {
        self.infopanel_visible.update(|v| *v = !*v);
    }

    /// Set the sidebar visibility explicitly
    pub fn set_sidebar_visible(&self, visible: bool) {
        self.sidebar_visible.set(visible);
    }

    /// Set the info panel visibility explicitly
    pub fn set_infopanel_visible(&self, visible: bool) {
        self.infopanel_visible.set(visible);
    }

    /// Set the sidebar width (clamped to reasonable bounds)
    pub fn set_sidebar_width(&self, width: i32) {
        let clamped = width.clamp(200, 500);
        self.sidebar_width.set(clamped);
    }

    /// Set the info panel width (clamped to reasonable bounds)
    pub fn set_infopanel_width(&self, width: i32) {
        let clamped = width.clamp(200, 600);
        self.infopanel_width.set(clamped);
    }

    /// Navigate to a specific view
    pub fn navigate_to(&self, view: ViewType) {
        self.active_view.set(view);
    }

    /// Check if a specific view is currently active
    pub fn is_active(&self, view: ViewType) -> bool {
        self.active_view.get() == view
    }
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide the LayoutState to the component tree via context.
///
/// Call this function in your root component (e.g., App) to make
/// LayoutState available to all child components.
///
/// # Example
/// ```rust,ignore
/// #[component]
/// pub fn App() -> impl IntoView {
///     provide_layout_state();
///     // ... rest of your app
/// }
/// ```
pub fn provide_layout_state() {
    provide_context(LayoutState::new());
}

/// Retrieve the LayoutState from context.
///
/// Panics if LayoutState has not been provided via `provide_layout_state()`.
///
/// # Example
/// ```rust,ignore
/// #[component]
/// pub fn Sidebar() -> impl IntoView {
///     let layout = use_layout_state();
///     let visible = layout.sidebar_visible;
///     // ...
/// }
/// ```
pub fn use_layout_state() -> LayoutState {
    expect_context::<LayoutState>()
}

/// Try to retrieve the LayoutState from context, returning None if not provided.
pub fn try_use_layout_state() -> Option<LayoutState> {
    use_context::<LayoutState>()
}
