use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ResizeSide {
    Left,  // Sidebar: Handle is on its Right
    Right, // InfoPanel: Handle is on its Left
}

#[derive(Props, Clone, PartialEq)]
pub struct DragHandleProps {
    pub on_drag_start: EventHandler<()>,
    pub side: ResizeSide,
}

#[component]
pub fn DragHandle(props: DragHandleProps) -> Element {
    let position_class = match props.side {
        ResizeSide::Left => "right-[-2px]", // Overlap border
        ResizeSide::Right => "left-[-2px]",
    };

    rsx! {
        div {
            // Invisible hit area (larger) + Visible line
            class: "absolute top-0 bottom-0 select-none z-50 w-2 cursor-col-resize flex justify-center group {position_class}",
            onmousedown: move |e| {
                e.stop_propagation();
                props.on_drag_start.call(());
            },
            // Visible line on hover/active
            div {
                class: "w-[2px] h-full bg-transparent group-hover:bg-purple-500 transition-colors delay-75 active:bg-purple-600"
            }
        }
    }
}
