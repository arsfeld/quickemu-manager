mod main_window;
mod vm_card;
mod vm_create_dialog;
mod vm_edit_dialog;
mod settings_dialog;
// mod simple_vm_create_dialog;
// mod simple_settings_dialog;
// mod header_bar;  // No longer needed - using AdwHeaderBar directly

pub use main_window::MainWindow;
pub use vm_card::VMCard;
pub use vm_create_dialog::VMCreateDialog;
pub use vm_edit_dialog::VMEditDialog;
pub use settings_dialog::SettingsDialog;
// pub use header_bar::HeaderBar;  // No longer needed