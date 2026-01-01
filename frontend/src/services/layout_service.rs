use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct LayoutState {
    pub sidebar_visible: Signal<bool>,
    pub sidebar_width: Signal<i32>,
    pub infopanel_visible: Signal<bool>,
    pub infopanel_width: Signal<i32>,
    pub active_view: Signal<ViewType>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewType {
    Campaigns,
    Chat,
    Library,
    Graph,
    Settings,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            sidebar_visible: use_signal(|| true),
            sidebar_width: use_signal(|| 280),
            infopanel_visible: use_signal(|| true),
            infopanel_width: use_signal(|| 320),
            active_view: use_signal(|| ViewType::Campaigns),
        }
    }

    pub fn toggle_sidebar(&mut self) {
        let current = *self.sidebar_visible.read();
        self.sidebar_visible.set(!current);
    }

    pub fn toggle_infopanel(&mut self) {
        let current = *self.infopanel_visible.read();
        self.infopanel_visible.set(!current);
    }
}
