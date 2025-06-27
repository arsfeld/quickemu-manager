use gtk::prelude::*;
use gtk::{glib, Button, ProgressBar, TextView};
use adw::{prelude::*, ComboRow, EntryRow, SpinRow, ActionRow, PreferencesGroup, ExpanderRow, ViewStack, StatusPage, WindowTitle};
use adw::subclass::prelude::*;

use crate::AppState;
use quickemu_core::VMTemplate;

mod imp {
    use super::*;
    use std::cell::RefCell;
    use gtk::{CompositeTemplate, TemplateChild};

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/quickemu/Manager/ui/vm_create_dialog.ui")]
    pub struct VMCreateDialog {
        // Header elements
        #[template_child]
        pub window_title: TemplateChild<WindowTitle>,
        #[template_child]
        pub back_button: TemplateChild<Button>,
        #[template_child]
        pub cancel_button: TemplateChild<Button>,
        #[template_child]
        pub action_button: TemplateChild<Button>,
        
        // Main stack
        #[template_child]
        pub main_stack: TemplateChild<ViewStack>,
        
        // Configuration page elements
        #[template_child]
        pub name_entry: TemplateChild<EntryRow>,
        #[template_child]
        pub os_row: TemplateChild<ComboRow>,
        #[template_child]
        pub version_row: TemplateChild<ComboRow>,
        #[template_child]
        pub edition_row: TemplateChild<ComboRow>,
        #[template_child]
        pub ram_row: TemplateChild<ComboRow>,
        #[template_child]
        pub cpu_row: TemplateChild<SpinRow>,
        #[template_child]
        pub disk_row: TemplateChild<ComboRow>,
        
        // Progress page elements
        #[template_child]
        pub progress_page: TemplateChild<StatusPage>,
        #[template_child]
        pub progress_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub console_expander: TemplateChild<ExpanderRow>,
        #[template_child]
        pub console_view: TemplateChild<TextView>,
        
        // Complete page elements
        #[template_child]
        pub complete_page: TemplateChild<StatusPage>,
        #[template_child]
        pub start_vm_button: TemplateChild<Button>,
        #[template_child]
        pub done_button: TemplateChild<Button>,
        
        pub app_state: RefCell<Option<AppState>>,
        pub current_template: RefCell<Option<VMTemplate>>,
        pub os_data: RefCell<Vec<quickemu_core::OSInfo>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMCreateDialog {
        const NAME: &'static str = "QEMVMCreateDialog";
        type Type = super::VMCreateDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VMCreateDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for VMCreateDialog {}
    impl WindowImpl for VMCreateDialog {}
    impl AdwWindowImpl for VMCreateDialog {}

    impl Default for VMCreateDialog {
        fn default() -> Self {
            Self {
                window_title: TemplateChild::default(),
                back_button: TemplateChild::default(),
                cancel_button: TemplateChild::default(),
                action_button: TemplateChild::default(),
                main_stack: TemplateChild::default(),
                name_entry: TemplateChild::default(),
                os_row: TemplateChild::default(),
                version_row: TemplateChild::default(),
                edition_row: TemplateChild::default(),
                ram_row: TemplateChild::default(),
                cpu_row: TemplateChild::default(),
                disk_row: TemplateChild::default(),
                progress_page: TemplateChild::default(),
                progress_bar: TemplateChild::default(),
                console_expander: TemplateChild::default(),
                console_view: TemplateChild::default(),
                complete_page: TemplateChild::default(),
                start_vm_button: TemplateChild::default(),
                done_button: TemplateChild::default(),
                app_state: RefCell::new(None),
                current_template: RefCell::new(None),
                os_data: RefCell::new(Vec::new()),
            }
        }
    }
}

glib::wrapper! {
    pub struct VMCreateDialog(ObjectSubclass<imp::VMCreateDialog>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl VMCreateDialog {
    pub fn new(parent: &impl IsA<gtk::Window>, app_state: AppState) -> Self {
        let dialog: Self = glib::Object::builder()
            .property("transient-for", parent)
            .build();
        
        let imp = dialog.imp();
        imp.app_state.replace(Some(app_state.clone()));
        
        // Set initial state
        dialog.setup_wizard();
        
        dialog
    }

    pub fn present(&self) {
        self.set_visible(true);
    }
    
    fn setup_wizard(&self) {
        let imp = self.imp();
        
        // Set initial page
        imp.main_stack.set_visible_child_name("config");
        imp.action_button.set_label("Create VM");
        imp.back_button.set_visible(false);
        
        // Load OS and version data from quickget
        self.load_os_data();
        
        // Connect cancel button
        let dialog_weak = self.downgrade();
        imp.cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });
        
        // Connect back button
        let dialog_weak = self.downgrade();
        imp.back_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.go_back();
            }
        });
        
        // Connect action button (context-sensitive)
        let dialog_weak = self.downgrade();
        imp.action_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.handle_action_button();
            }
        });
        
        // Connect completion buttons
        let dialog_weak = self.downgrade();
        imp.done_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });
        
        let dialog_weak = self.downgrade();
        imp.start_vm_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                // TODO: Implement start VM functionality
                dialog.close();
            }
        });
    }
    
    fn load_os_data(&self) {
        let imp = self.imp();
        let app_state = imp.app_state.borrow().as_ref().unwrap().clone();
        
        // First, populate with popular systems
        if let Some(quickget_service) = &app_state.quickget_service {
            let dialog_weak = self.downgrade();
            let quickget_service = quickget_service.clone();
            
            glib::spawn_future_local(async move {
                match quickget_service.get_popular_systems().await {
                    Ok(popular_systems) => {
                        if let Some(dialog) = dialog_weak.upgrade() {
                            let imp = dialog.imp();
                            
                            // Clear existing items and populate OS dropdown
                            let os_model = gtk::StringList::new(&[]);
                            
                            for os in &popular_systems {
                                os_model.append(&os.name);
                            }
                            
                            imp.os_row.set_model(Some(&os_model));
                            
                            // Store the OS data for version lookup
                            dialog.store_os_data(popular_systems);
                            
                            // Load versions for the first OS
                            if os_model.n_items() > 0 {
                                dialog.load_versions_for_os(0);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load OS data from quickget: {}", e);
                        // Show error state - no fallback
                        if let Some(dialog) = dialog_weak.upgrade() {
                            dialog.show_quickget_error();
                        }
                    }
                }
            });
        } else {
            // No quickget service available - show error
            self.show_quickget_error();
        }
        
        // Setup OS row change handler
        let dialog_weak = self.downgrade();
        imp.os_row.connect_selected_notify(move |combo_row| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.load_versions_for_os(combo_row.selected());
                dialog.update_vm_name();
            }
        });
        
        // Setup version row change handler
        let dialog_weak = self.downgrade();
        imp.version_row.connect_selected_notify(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.update_vm_name();
            }
        });
        
        // Setup edition row change handler
        let dialog_weak = self.downgrade();
        imp.edition_row.connect_selected_notify(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.update_vm_name();
            }
        });
    }
    
    fn store_os_data(&self, os_data: Vec<quickemu_core::OSInfo>) {
        let imp = self.imp();
        imp.os_data.replace(os_data);
    }
    
    fn get_stored_os_data(&self) -> Option<Vec<quickemu_core::OSInfo>> {
        let imp = self.imp();
        let data = imp.os_data.borrow();
        if data.is_empty() {
            None
        } else {
            Some(data.clone())
        }
    }
    
    fn load_versions_for_os(&self, os_index: u32) {
        let imp = self.imp();
        
        if let Some(os_data) = self.get_stored_os_data() {
            if let Some(os_info) = os_data.get(os_index as usize) {
                // Clear existing versions
                let version_model = gtk::StringList::new(&[]);
                
                // Add versions for this OS
                for version in &os_info.versions {
                    version_model.append(version);
                }
                
                imp.version_row.set_model(Some(&version_model));
                
                // Select first version by default
                if version_model.n_items() > 0 {
                    imp.version_row.set_selected(0);
                }
                
                // Load editions for this OS
                self.load_editions_for_os(&os_info.name);
                
                // Update VM name after loading versions
                self.update_vm_name();
            }
        }
    }
    
    fn load_editions_for_os(&self, os_name: &str) {
        let imp = self.imp();
        let app_state = imp.app_state.borrow().as_ref().unwrap().clone();
        
        // Hide edition row by default
        imp.edition_row.set_visible(false);
        
        if let Some(quickget_service) = &app_state.quickget_service {
            let dialog_weak = self.downgrade();
            let quickget_service = quickget_service.clone();
            let os_name = os_name.to_string();
            
            glib::spawn_future_local(async move {
                match quickget_service.get_editions(&os_name).await {
                    Ok(editions) => {
                        if let Some(dialog) = dialog_weak.upgrade() {
                            let imp = dialog.imp();
                            
                            if !editions.is_empty() {
                                // Show edition row and populate it
                                let edition_model = gtk::StringList::new(&[]);
                                for edition in &editions {
                                    edition_model.append(edition);
                                }
                                
                                imp.edition_row.set_model(Some(&edition_model));
                                imp.edition_row.set_selected(0);
                                imp.edition_row.set_visible(true);
                                
                                // Set preferred default edition for known OSes
                                match os_name.as_str() {
                                    "fedora" => {
                                        // Default to Workstation for Fedora
                                        for (i, edition) in editions.iter().enumerate() {
                                            if edition == "Workstation" {
                                                imp.edition_row.set_selected(i as u32);
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                
                                // Update VM name after edition is set
                                if let Some(dialog) = dialog_weak.upgrade() {
                                    dialog.update_vm_name();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load editions for {}: {}", os_name, e);
                    }
                }
            });
        }
    }
    
    fn update_vm_name(&self) {
        let imp = self.imp();
        
        // Get current selections
        let os_name = if let Some(os_data) = self.get_stored_os_data() {
            let os_index = imp.os_row.selected() as usize;
            os_data.get(os_index).map(|os| os.name.clone())
        } else {
            None
        };
        
        let version = if let Some(model) = imp.version_row.model() {
            if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                let version_index = imp.version_row.selected() as u32;
                if version_index < string_list.n_items() {
                    string_list.string(version_index).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let edition = if imp.edition_row.is_visible() {
            if let Some(model) = imp.edition_row.model() {
                if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                    let edition_index = imp.edition_row.selected() as u32;
                    if edition_index < string_list.n_items() {
                        string_list.string(edition_index).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Generate VM name
        if let (Some(os), Some(ver)) = (os_name, version) {
            let vm_name = if let Some(ed) = edition {
                // For OSes with editions, include edition in name
                format!("{}-{}-{}", os, ver, ed.to_lowercase())
            } else {
                // For simple OSes, just use OS and version
                format!("{}-{}", os, ver)
            };
            
            // Only update if the current text is empty or matches a previous auto-generated pattern
            let current_text = imp.name_entry.text().to_string();
            if current_text.is_empty() || self.is_auto_generated_name(&current_text) {
                imp.name_entry.set_text(&vm_name);
            }
        }
    }
    
    fn is_auto_generated_name(&self, name: &str) -> bool {
        // Check if the name follows the pattern of OS-version or OS-version-edition
        let parts: Vec<&str> = name.split('-').collect();
        
        // Should have at least 2 parts (os-version) or 3 parts (os-version-edition)
        if parts.len() < 2 || parts.len() > 3 {
            return false;
        }
        
        // Check if it matches current OS data
        if let Some(os_data) = self.get_stored_os_data() {
            let imp = self.imp();
            let os_index = imp.os_row.selected() as usize;
            
            if let Some(os_info) = os_data.get(os_index) {
                // First part should match OS name
                if parts[0] != os_info.name {
                    return false;
                }
                
                // Second part should be a valid version
                if !os_info.versions.contains(&parts[1].to_string()) {
                    return false;
                }
                
                // If there's a third part, it should be an edition (if editions are visible)
                if parts.len() == 3 && imp.edition_row.is_visible() {
                    // We could check if it's a valid edition, but for simplicity just accept it
                    return true;
                } else if parts.len() == 2 && !imp.edition_row.is_visible() {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn show_quickget_error(&self) {
        let imp = self.imp();
        
        // Clear dropdowns and disable form
        imp.os_row.set_model(None::<&gtk::StringList>);
        imp.version_row.set_model(None::<&gtk::StringList>);
        imp.action_button.set_sensitive(false);
        
        // Show error message
        imp.os_row.set_subtitle("Quickget not available - cannot create VMs");
        imp.version_row.set_subtitle("Install quickget/quickemu to enable VM creation");
        
        // TODO: Add button to download/install quickget
    }
    
    fn handle_action_button(&self) {
        let imp = self.imp();
        let current_page = imp.main_stack.visible_child_name().unwrap_or_default();
        
        match current_page.as_str() {
            "config" => self.start_vm_creation(),
            _ => {}
        }
    }
    
    fn go_back(&self) {
        let imp = self.imp();
        let current_page = imp.main_stack.visible_child_name().unwrap_or_default();
        
        match current_page.as_str() {
            "progress" => {
                imp.main_stack.set_visible_child_name("config");
                imp.action_button.set_label("Create VM");
                imp.back_button.set_visible(false);
            }
            "complete" => {
                imp.main_stack.set_visible_child_name("config");
                imp.action_button.set_label("Create VM");
                imp.back_button.set_visible(false);
            }
            _ => {}
        }
    }
    
    fn start_vm_creation(&self) {
        let imp = self.imp();
        let app_state = imp.app_state.borrow().as_ref().unwrap().clone();
        
        // Get form values
        let name = imp.name_entry.text().to_string();
        if name.is_empty() {
            return;
        }
        
        // Get selected OS, version, and edition from stored data
        let (os, version, edition) = if let Some(os_data) = self.get_stored_os_data() {
            let os_index = imp.os_row.selected() as usize;
            let version_index = imp.version_row.selected() as usize;
            
            if let Some(os_info) = os_data.get(os_index) {
                let os_name = os_info.name.clone();
                let version_name = os_info.versions.get(version_index)
                    .cloned()
                    .unwrap_or_else(|| "latest".to_string());
                
                // Get edition if available and visible
                let edition = if imp.edition_row.is_visible() {
                    if let Some(model) = imp.edition_row.model() {
                        if let Some(string_list) = model.downcast_ref::<gtk::StringList>() {
                            let edition_index = imp.edition_row.selected() as usize;
                            if edition_index < string_list.n_items() as usize {
                                string_list.string(edition_index as u32)
                                    .map(|s| s.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                (os_name, version_name, edition)
            } else {
                ("ubuntu".to_string(), "22.04".to_string(), None)
            }
        } else {
            ("ubuntu".to_string(), "22.04".to_string(), None)
        };
        
        // Hardware configuration options
        let ram_options = vec!["2G", "4G", "8G", "16G", "32G"];
        let disk_options = vec!["32G", "64G", "128G", "256G", "512G"];
        
        let ram = ram_options.get(imp.ram_row.selected() as usize).unwrap_or(&"4G").to_string();
        let cpu_cores = imp.cpu_row.value() as u32;
        let disk_size = disk_options.get(imp.disk_row.selected() as usize).unwrap_or(&"64G").to_string();
        
        let template = VMTemplate {
            name: name.clone(),
            os: os.clone(),
            version: version.clone(),
            edition: edition.clone(),
            ram,
            disk_size,
            cpu_cores,
        };
        
        // Store template for later reference
        imp.current_template.replace(Some(template.clone()));
        
        // Animate to progress page
        imp.main_stack.set_visible_child_name("progress");
        imp.action_button.set_sensitive(false);
        imp.back_button.set_visible(true);
        imp.window_title.set_title("Creating VM");
        
        // Update progress page with VM details
        imp.progress_page.set_title(&format!("Creating {}", name));
        let description = if let Some(ref edition) = edition {
            format!("Setting up {} {} {} with {} RAM", os, version, edition, template.ram)
        } else {
            format!("Setting up {} {} with {} RAM", os, version, template.ram)
        };
        imp.progress_page.set_description(Some(&description));
        
        // Clear and setup console view
        let console_buffer = imp.console_view.buffer();
        console_buffer.set_text("");
        
        // Start progress bar animation
        imp.progress_bar.set_pulse_step(0.1);
        let progress_bar = imp.progress_bar.clone();
        let pulse_id = std::rc::Rc::new(std::cell::RefCell::new(Some(
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                progress_bar.pulse();
                glib::ControlFlow::Continue
            })
        )));
        
        // Start VM creation
        let dialog_weak = self.downgrade();
        let pulse_id_clone = pulse_id.clone();
        
        glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
            Self::create_vm_with_progress(template, app_state, dialog_weak.clone(), move |success| {
                if let Some(dialog) = dialog_weak.upgrade() {
                    
                    // Stop progress animation
                    if let Some(id) = pulse_id_clone.borrow_mut().take() {
                        id.remove();
                    }
                    
                    if success {
                        dialog.show_completion_page(true);
                    } else {
                        dialog.show_completion_page(false);
                    }
                }
            });
        });
    }
    
    fn show_completion_page(&self, success: bool) {
        let imp = self.imp();
        
        if success {
            // Show success page
            imp.main_stack.set_visible_child_name("complete");
            imp.action_button.set_visible(false);
            imp.back_button.set_visible(false);
            imp.cancel_button.set_label("Close");
            imp.window_title.set_title("VM Created");
            
            let template = imp.current_template.borrow();
            if let Some(ref template) = *template {
                imp.complete_page.set_title(&format!("{} is ready!", template.name));
                imp.complete_page.set_description(Some("Your virtual machine has been created successfully"));
            }
        } else {
            // Stay on progress page but show error state
            imp.progress_page.set_icon_name(Some("dialog-error-symbolic"));
            imp.progress_page.set_title("Creation Failed");
            imp.progress_page.set_description(Some("There was an error creating your VM. Check the console output for details."));
            imp.action_button.set_visible(false);
            imp.back_button.set_visible(true);
            imp.progress_bar.set_visible(false);
        }
    }
    
    fn create_vm_with_progress<F>(template: VMTemplate, app_state: AppState, dialog_weak: glib::WeakRef<VMCreateDialog>, callback: F) 
    where 
        F: Fn(bool) + 'static
    {
        println!("=== Creating VM: {} ===", template.name);
        println!("OS: {} {}", template.os, template.version);
        println!("RAM: {}, CPU Cores: {}, Disk: {}", template.ram, template.cpu_cores, template.disk_size);
        
        glib::spawn_future_local(async move {
            let target_dir = app_state.config_manager.get_primary_vm_directory().await
                .join(&template.name);

            println!("Target directory: {}", target_dir.display());
            
            // Start VM creation with real-time output
            let output_receiver: std::sync::mpsc::Receiver<String> = match app_state.vm_manager.spawn_vm_creation_with_output(template, target_dir) {
                Ok(rx) => rx,
                Err(e) => {
                    eprintln!("❌ Failed to start VM creation: {}", e);
                    callback(false);
                    return;
                }
            };
            
            // Set up a timer to poll for output messages
            let dialog_weak_for_polling = dialog_weak.clone();
            let callback_clone = std::rc::Rc::new(std::cell::RefCell::new(Some(callback)));
            let receiver = std::rc::Rc::new(std::cell::RefCell::new(output_receiver));
            
            glib::timeout_add_local(std::time::Duration::from_millis(100), {
                let dialog_weak = dialog_weak_for_polling;
                let callback_clone = callback_clone.clone();
                let receiver = receiver.clone();
                
                move || {
                    // Try to receive messages from the worker thread
                    let mut completion_status = None;
                    let mut channel_closed = false;
                    
                    // Process all available messages
                    if let Ok(receiver_ref) = receiver.try_borrow_mut() {
                        loop {
                            match receiver_ref.try_recv() {
                                Ok(output) => {
                                    // Update UI with the output
                                    if let Some(dialog) = dialog_weak.upgrade() {
                                        let imp = dialog.imp();
                                        let buffer = imp.console_view.buffer();
                                        let mut end_iter = buffer.end_iter();
                                        buffer.insert(&mut end_iter, &output);
                                        
                                        // Auto-scroll to bottom
                                        let mark = buffer.create_mark(None, &buffer.end_iter(), false);
                                        imp.console_view.scroll_mark_onscreen(&mark);
                                    }
                                    
                                    // Check for completion
                                    if output.contains("VM created successfully") {
                                        completion_status = Some(true);
                                    } else if output.contains("Failed to create directory") || 
                                             output.contains("Failed to spawn quickget") ||
                                             output.contains("quickget failed with exit code") ||
                                             output.contains("Config file not created") ||
                                             output.contains("VM creation failed") {
                                        completion_status = Some(false);
                                    }
                                }
                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                    // No more messages available right now
                                    break;
                                }
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    // Channel is closed, VM creation thread has finished
                                    channel_closed = true;
                                    break;
                                }
                            }
                        }
                    }
                    
                    // Handle completion
                    if let Some(success) = completion_status {
                        if let Some(callback) = callback_clone.borrow_mut().take() {
                            callback(success);
                        }
                        return glib::ControlFlow::Break; // Stop the timer
                    }
                    
                    // If channel is closed but we didn't get a completion message, assume failure
                    if channel_closed {
                        if let Some(callback) = callback_clone.borrow_mut().take() {
                            callback(false);
                        }
                        return glib::ControlFlow::Break;
                    }
                    
                    glib::ControlFlow::Continue // Keep polling
                }
            });
        });
    }
}