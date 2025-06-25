use gtk::prelude::*;
use adw::prelude::*;

use crate::AppState;
use quickemu_core::{VM, ConfigParser};

pub struct VMEditDialog {
    dialog: adw::Window,
    vm: VM,
    app_state: AppState,
    // Form fields
    name_entry: gtk::Entry,
    ram_entry: gtk::Entry,
    cpu_spin: gtk::SpinButton,
    disk_size_entry: gtk::Entry,
}

impl VMEditDialog {
    pub fn new(parent: &impl IsA<gtk::Window>, vm: VM, app_state: AppState) -> Self {
        let dialog = adw::Window::builder()
            .title(&format!("Edit VM: {}", vm.name))
            .modal(true)
            .transient_for(parent)
            .default_width(600)
            .default_height(400)
            .build();

        let header_bar = adw::HeaderBar::new();
        
        let cancel_button = gtk::Button::builder()
            .label("Cancel")
            .build();
        
        let save_button = gtk::Button::builder()
            .label("Save")
            .css_classes(["suggested-action"])
            .build();
        
        header_bar.pack_start(&cancel_button);
        header_bar.pack_end(&save_button);

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        main_box.append(&header_bar);

        // Create preference groups
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);

        // Basic Settings Group
        let basic_group = adw::PreferencesGroup::builder()
            .title("Basic Settings")
            .build();

        // Name row
        let name_entry = gtk::Entry::builder()
            .text(&vm.name)
            .sensitive(false) // VM name shouldn't be changed as it's tied to the filename
            .build();
        
        let name_row = adw::ActionRow::builder()
            .title("VM Name")
            .build();
        name_row.add_suffix(&name_entry);
        basic_group.add(&name_row);

        // Guest OS row (read-only)
        let os_label = gtk::Label::builder()
            .label(&vm.config.guest_os)
            .build();
        
        let os_row = adw::ActionRow::builder()
            .title("Guest OS")
            .build();
        os_row.add_suffix(&os_label);
        basic_group.add(&os_row);

        content_box.append(&basic_group);

        // Resources Group
        let resources_group = adw::PreferencesGroup::builder()
            .title("Resources")
            .build();

        // RAM row
        let ram_entry = gtk::Entry::builder()
            .text(&vm.config.ram)
            .placeholder_text("e.g., 2G, 4096M")
            .build();
        
        let ram_row = adw::ActionRow::builder()
            .title("RAM")
            .subtitle("Amount of memory (e.g., 2G, 4096M)")
            .build();
        ram_row.add_suffix(&ram_entry);
        resources_group.add(&ram_row);

        // CPU cores row
        let cpu_adjustment = gtk::Adjustment::builder()
            .lower(1.0)
            .upper(32.0)
            .step_increment(1.0)
            .value(vm.config.cpu_cores as f64)
            .build();
        
        let cpu_spin = gtk::SpinButton::builder()
            .adjustment(&cpu_adjustment)
            .climb_rate(1.0)
            .digits(0)
            .build();
        
        let cpu_row = adw::ActionRow::builder()
            .title("CPU Cores")
            .subtitle("Number of virtual CPU cores")
            .build();
        cpu_row.add_suffix(&cpu_spin);
        resources_group.add(&cpu_row);

        // Disk size row
        let disk_size_entry = gtk::Entry::builder()
            .text(vm.config.disk_size.as_deref().unwrap_or(""))
            .placeholder_text("e.g., 20G, 100G")
            .build();
        
        let disk_row = adw::ActionRow::builder()
            .title("Disk Size")
            .subtitle("Virtual disk size (e.g., 20G)")
            .build();
        disk_row.add_suffix(&disk_size_entry);
        resources_group.add(&disk_row);

        content_box.append(&resources_group);

        // Wrap in scrolled window
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&content_box)
            .build();

        main_box.append(&scrolled);
        dialog.set_content(Some(&main_box));

        let dialog_struct = Self {
            dialog: dialog.clone(),
            vm: vm.clone(),
            app_state: app_state.clone(),
            name_entry,
            ram_entry: ram_entry.clone(),
            cpu_spin: cpu_spin.clone(),
            disk_size_entry: disk_size_entry.clone(),
        };

        // Connect cancel button
        let dialog_weak = dialog.downgrade();
        cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });

        // Connect save button
        let dialog_weak = dialog.downgrade();
        let vm_clone = vm.clone();
        let app_state_clone = app_state.clone();
        let ram_entry_clone = ram_entry.clone();
        let cpu_spin_clone = cpu_spin.clone();
        let disk_size_entry_clone = disk_size_entry.clone();
        
        save_button.connect_clicked(move |_| {
            let mut updated_config = vm_clone.config.clone();
            
            // Update configuration values
            updated_config.ram = ram_entry_clone.text().to_string();
            updated_config.cpu_cores = cpu_spin_clone.value() as u32;
            
            let disk_size_text = disk_size_entry_clone.text();
            if !disk_size_text.is_empty() {
                updated_config.disk_size = Some(disk_size_text.to_string());
            }
            
            // Save the configuration
            if let Err(e) = ConfigParser::save_config(&vm_clone.config_path, &updated_config) {
                eprintln!("Failed to save VM configuration: {}", e);
                // TODO: Show error dialog
            } else {
                // Close dialog on successful save
                if let Some(dialog) = dialog_weak.upgrade() {
                    dialog.close();
                }
                
                // TODO: Trigger refresh of VM list
            }
        });

        dialog_struct
    }

    pub fn present(&self) {
        self.dialog.present();
    }
}