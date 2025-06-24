use dioxus::prelude::*;
use crate::models::{VM, VMId};
use crate::components::VMCard;

#[component]
pub fn VMList(
    vms: Vec<VM>,
    on_vm_start: EventHandler<VMId>,
    on_vm_stop: EventHandler<VMId>,
    on_vm_select: EventHandler<VMId>,
) -> Element {
    if vms.is_empty() {
        return rsx! {
            div { class: "vm-list-empty",
                h2 { "No Virtual Machines Found" }
                p { "Create a new VM or add existing VM configurations to get started." }
            }
        };
    }
    
    rsx! {
        div { class: "vm-list",
            for vm in vms.iter() {
                VMCard {
                    key: "{vm.id.0}",
                    vm: vm.clone(),
                    on_start: move |id| on_vm_start(id),
                    on_stop: move |id| on_vm_stop(id),
                    on_click: move |id| on_vm_select(id),
                }
            }
        }
    }
}