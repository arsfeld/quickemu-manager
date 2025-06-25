pub mod vm_manager;

pub use quickemu_core::{VM, VMStatus, VMConfig};

#[cfg(feature = "desktop")]
pub use vm_manager::DesktopVMManager as VMManager;

#[cfg(feature = "server")]
pub use vm_manager::ServerVMManager as VMManager;