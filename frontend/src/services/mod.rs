pub mod layout_service;

use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct DragState(pub Signal<Option<String>>);
