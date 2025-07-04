use dioxus::prelude::*;

/// Keyboard shortcuts handler for desktop
#[component]
pub fn KeyboardShortcuts(
    on_new_vm: EventHandler<()>,
    on_settings: EventHandler<()>,
    on_refresh: EventHandler<()>,
) -> Element {
    // Keyboard shortcuts are not available in this version of dioxus
    // TODO: Implement keyboard shortcuts when available

    rsx! {}
}
