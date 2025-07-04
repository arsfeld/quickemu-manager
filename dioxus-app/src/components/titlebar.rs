use dioxus::prelude::*;

/// Native-style titlebar component
#[component]
pub fn Titlebar() -> Element {
    let mut is_maximized = use_signal(|| false);

    rsx! {
        div {
            class: "h-12 bg-macos-surface border-b border-macos-border flex items-center justify-between px-4 select-none",
            style: "-webkit-app-region: drag; app-region: drag;",

            // Window controls (macOS style)
            div {
                class: "flex items-center gap-2",
                style: "-webkit-app-region: no-drag; app-region: no-drag;",

                button {
                    class: "w-3 h-3 rounded-full bg-macos-red-500 hover:bg-macos-red-600 transition-colors",
                    onclick: move |_| {
                        // TODO: Implement window close
                    }
                }
                button {
                    class: "w-3 h-3 rounded-full bg-macos-yellow-500 hover:bg-macos-yellow-600 transition-colors",
                    onclick: move |_| {
                        // TODO: Implement window minimize
                    }
                }
                button {
                    class: "w-3 h-3 rounded-full bg-macos-green-500 hover:bg-macos-green-600 transition-colors",
                    onclick: move |_| {
                        // TODO: Implement window maximize/unmaximize
                        is_maximized.toggle();
                    }
                }
            }

            // Title
            div {
                class: "absolute left-1/2 transform -translate-x-1/2",
                h1 {
                    class: "text-sm font-medium text-macos-text",
                    "Quickemu Manager"
                }
            }

            // Right side (empty for balance)
            div { class: "w-16" }
        }
    }
}
