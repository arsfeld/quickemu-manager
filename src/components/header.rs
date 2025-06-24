use dioxus::prelude::*;

#[component]
pub fn Header(
    on_create_vm: EventHandler<()>,
    on_settings: EventHandler<()>,
    on_refresh: EventHandler<()>,
) -> Element {
    rsx! {
        header { class: "app-header",
            div { class: "header-left",
                h1 { "Quickemu Manager" }
            }
            
            div { class: "header-right",
                button {
                    class: "btn btn-primary",
                    onclick: move |_| on_create_vm(()),
                    "➕ Create VM"
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| on_refresh(()),
                    "🔄 Refresh"
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| on_settings(()),
                    "⚙ Settings"
                }
            }
        }
    }
}