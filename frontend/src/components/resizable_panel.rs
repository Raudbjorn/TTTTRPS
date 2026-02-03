use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ResizeSide {
    Left,  // Sidebar: Handle is on its Right
    Right, // InfoPanel: Handle is on its Left
}

#[component]
pub fn DragHandle(side: ResizeSide, #[prop(into)] on_drag_start: Callback<()>) -> impl IntoView {
    let position_class = match side {
        ResizeSide::Left => "right-[-2px]",
        ResizeSide::Right => "left-[-2px]",
    };

    view! {
        <div
            class=format!(
                "absolute top-0 bottom-0 select-none z-50 w-2 cursor-col-resize flex justify-center group {}",
                position_class
            )
            on:mousedown=move |e| {
                e.stop_propagation();
                on_drag_start.run(());
            }
        >
            // Visible line on hover/active
            <div class="w-[2px] h-full bg-transparent group-hover:bg-purple-500 transition-colors delay-75 active:bg-purple-600"></div>
        </div>
    }
}
