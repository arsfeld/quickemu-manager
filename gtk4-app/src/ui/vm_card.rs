use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, TemplateChild};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use super::{MainWindow, VMEditDialog};
use crate::AppState;
use quickemu_core::{DisplayProtocol, VMStatus, VM};

mod imp {
    use super::*;

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/quickemu/Manager/ui/vm_card.ui")]
    pub struct VMCard {
        #[template_child]
        pub vm_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub vm_os: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub cpu_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub ram_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub disk_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub metrics_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub click_gesture: TemplateChild<gtk::GestureClick>,
        #[template_child]
        pub right_click_gesture: TemplateChild<gtk::GestureClick>,
        #[template_child]
        pub hover_controller: TemplateChild<gtk::EventControllerMotion>,
        #[template_child]
        pub clickable_area: TemplateChild<gtk::Box>,
        #[template_child]
        pub status_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub console_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub control_area: TemplateChild<gtk::Box>,

        pub vm: Rc<RefCell<Option<VM>>>,
        pub app_state: Rc<RefCell<Option<AppState>>>,
        pub process_monitor: Rc<RefCell<Option<Arc<quickemu_core::ProcessMonitor>>>>,
        pub start_time: Rc<RefCell<Option<std::time::Instant>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VMCard {
        const NAME: &'static str = "QEMVMCard";
        type Type = super::VMCard;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl VMCard {
        fn handle_click(&self) {
            let vm_ref = self.vm.borrow();
            let app_state_ref = self.app_state.borrow();
            
            if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                match &vm.status {
                    VMStatus::Stopped => {
                        // Start the VM when clicking on stopped VM
                        self.start_vm_and_open_console();
                    }
                    VMStatus::Running { .. } => {
                        // Open console for running VMs
                        if let Some(widget) = self.obj().root() {
                            if let Some(main_window) = widget.downcast_ref::<MainWindow>() {
                                main_window.show_vm_console(vm.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        fn start_vm_and_open_console(&self) {
            let vm_ref = self.vm.borrow();
            let app_state_ref = self.app_state.borrow();
            
            if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                if matches!(vm.status, VMStatus::Stopped) {
                    // Start the VM and open console
                    let vm_clone = vm.clone();
                    let app_state_clone = app_state.clone();
                    let obj_weak = self.obj().downgrade();
                    
                    glib::spawn_future_local(async move {
                        if let Err(e) = app_state_clone.vm_manager.start_vm(&vm_clone).await {
                            eprintln!("Failed to start VM: {}", e);
                        } else {
                            // Poll for VM to be running before opening console
                            let vm_id = vm_clone.id.clone();
                            let obj_weak_clone = obj_weak.clone();
                            let vm_clone2 = vm_clone.clone();
                            
                            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                                if let Some(obj) = obj_weak_clone.upgrade() {
                                    // Check if VM is now running
                                    let imp = obj.imp();
                                    if let Some(vm) = imp.vm.borrow().as_ref() {
                                        if matches!(vm.status, VMStatus::Running { .. }) {
                                            // VM is running, open console
                                            if let Some(widget) = obj.root() {
                                                if let Some(main_window) = widget.downcast_ref::<MainWindow>() {
                                                    main_window.show_vm_console(vm_clone2.clone());
                                                }
                                            }
                                            return glib::ControlFlow::Break;
                                        }
                                    }
                                }
                                // Keep polling
                                glib::ControlFlow::Continue
                            });
                        }
                    });
                }
            }
        }
        
        fn open_console(&self) {
            let vm_ref = self.vm.borrow();
            
            if let Some(vm) = vm_ref.as_ref() {
                if let Some(widget) = self.obj().root() {
                    if let Some(main_window) = widget.downcast_ref::<MainWindow>() {
                        main_window.show_vm_console(vm.clone());
                    }
                }
            }
        }
        
        fn show_context_menu(&self, x: f64, y: f64) {
            let menu = gio::Menu::new();
            
            // Edit section
            let edit_section = gio::Menu::new();
            edit_section.append(Some("Edit VM"), Some("vm.edit"));
            edit_section.append(Some("Delete VM"), Some("vm.delete"));
            menu.append_section(None, &edit_section);
            
            // Operations section  
            let ops_section = gio::Menu::new();
            ops_section.append(Some("Clone VM"), Some("vm.clone"));
            ops_section.append(Some("Export VM"), Some("vm.export"));
            menu.append_section(None, &ops_section);
            
            let popover = gtk::PopoverMenu::from_model(Some(&menu));
            popover.set_parent(&*self.obj());
            popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                x as i32,
                y as i32,
                1,
                1
            )));
            popover.popup();
        }
        
        fn handle_stop_button(&self) {
            let vm_ref = self.vm.borrow();
            let app_state_ref = self.app_state.borrow();
            
            if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                if matches!(vm.status, VMStatus::Running { .. }) {
                    let vm_id = vm.id.clone();
                    let app_state_clone = app_state.clone();
                    
                    glib::spawn_future_local(async move {
                        if let Err(e) = app_state_clone.vm_manager.stop_vm(&vm_id).await {
                            eprintln!("Failed to stop VM: {}", e);
                        }
                    });
                }
            }
        }
        
        fn setup_actions(&self) {
            let obj = self.obj();
            let obj_weak = obj.downgrade();
            
            // Edit action
            let edit_action = gio::SimpleAction::new("edit", None);
            let vm = self.vm.clone();
            let app_state = self.app_state.clone();
            edit_action.connect_activate(move |_, _| {
                let vm_ref = vm.borrow();
                let app_state_ref = app_state.borrow();
                
                if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                    if let Some(obj) = obj_weak.upgrade() {
                        if let Some(widget) = obj.root() {
                            if let Some(window) = widget.downcast_ref::<gtk::Window>() {
                                let dialog = VMEditDialog::new(window, vm.clone(), app_state.clone());
                                dialog.present();
                            }
                        }
                    }
                }
            });
            
            // Delete action
            let delete_action = gio::SimpleAction::new("delete", None);
            let vm = self.vm.clone();
            delete_action.connect_activate(move |_, _| {
                let vm_ref = vm.borrow();
                if let Some(vm) = vm_ref.as_ref() {
                    eprintln!("Delete VM not yet implemented for: {}", vm.name);
                }
            });
            
            // Clone action
            let clone_action = gio::SimpleAction::new("clone", None);
            let vm = self.vm.clone();
            clone_action.connect_activate(move |_, _| {
                let vm_ref = vm.borrow();
                if let Some(vm) = vm_ref.as_ref() {
                    eprintln!("Clone VM not yet implemented for: {}", vm.name);
                }
            });
            
            // Export action
            let export_action = gio::SimpleAction::new("export", None);
            let vm = self.vm.clone();
            export_action.connect_activate(move |_, _| {
                let vm_ref = vm.borrow();
                if let Some(vm) = vm_ref.as_ref() {
                    eprintln!("Export VM not yet implemented for: {}", vm.name);
                }
            });
            
            // Create action group
            let action_group = gio::SimpleActionGroup::new();
            action_group.add_action(&edit_action);
            action_group.add_action(&delete_action);
            action_group.add_action(&clone_action);
            action_group.add_action(&export_action);
            
            self.obj().insert_action_group("vm", Some(&action_group));
        }
    }

    impl ObjectImpl for VMCard {
        fn constructed(&self) {
            self.parent_constructed();
            
            // Set up actions
            self.setup_actions();
            
            // Add gestures to clickable area only
            let clickable_area = self.clickable_area.get();
            clickable_area.add_controller(self.click_gesture.get());
            clickable_area.add_controller(self.right_click_gesture.get());
            clickable_area.add_controller(self.hover_controller.get());
            
            // Connect left click gesture
            let click_gesture = self.click_gesture.get();
            click_gesture.connect_released(glib::clone!(@weak self as imp => move |_, _, _, _| {
                imp.handle_click();
            }));
            
            // Connect right click gesture for context menu
            let right_click_gesture = self.right_click_gesture.get();
            right_click_gesture.connect_released(glib::clone!(@weak self as imp => move |_, _, x, y| {
                imp.show_context_menu(x, y);
            }));
            
            // Add hover effects to clickable area
            let hover_controller = self.hover_controller.get();
            let clickable_area_weak = clickable_area.downgrade();
            
            hover_controller.connect_enter(move |_, _, _| {
                if let Some(area) = clickable_area_weak.upgrade() {
                    area.add_css_class("hover");
                }
            });
            
            let clickable_area_weak = clickable_area.downgrade();
            hover_controller.connect_leave(move |_| {
                if let Some(area) = clickable_area_weak.upgrade() {
                    area.remove_css_class("hover");
                }
            });
            
            // Set cursor to pointer on hover for clickable area
            clickable_area.set_cursor_from_name(Some("pointer"));
            
            // Connect status button (same as clicking the card when stopped)
            let status_button = self.status_button.get();
            status_button.connect_clicked(glib::clone!(@weak self as imp => move |_| {
                imp.start_vm_and_open_console();
            }));
            
            // Connect console button
            let console_button = self.console_button.get();
            console_button.connect_clicked(glib::clone!(@weak self as imp => move |_| {
                imp.open_console();
            }));
            
            // Connect stop button
            let stop_button = self.stop_button.get();
            stop_button.connect_clicked(glib::clone!(@weak self as imp => move |_| {
                imp.handle_stop_button();
            }));
        }
    }

    impl WidgetImpl for VMCard {}
    impl BoxImpl for VMCard {}

    impl Default for VMCard {
        fn default() -> Self {
            Self {
                vm_name: TemplateChild::default(),
                vm_os: TemplateChild::default(),
                status_icon: TemplateChild::default(),
                status_label: TemplateChild::default(),
                cpu_value: TemplateChild::default(),
                ram_value: TemplateChild::default(),
                disk_value: TemplateChild::default(),
                metrics_box: TemplateChild::default(),
                click_gesture: TemplateChild::default(),
                right_click_gesture: TemplateChild::default(),
                hover_controller: TemplateChild::default(),
                clickable_area: TemplateChild::default(),
                status_button: TemplateChild::default(),
                console_button: TemplateChild::default(),
                stop_button: TemplateChild::default(),
                control_area: TemplateChild::default(),
                vm: Rc::new(RefCell::new(None)),
                app_state: Rc::new(RefCell::new(None)),
                process_monitor: Rc::new(RefCell::new(None)),
                start_time: Rc::new(RefCell::new(None)),
            }
        }
    }
}

glib::wrapper! {
    pub struct VMCard(ObjectSubclass<imp::VMCard>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl VMCard {
    pub fn new(vm: VM, app_state: AppState, process_monitor: Option<Arc<quickemu_core::ProcessMonitor>>) -> Self {
        let card: Self = glib::Object::builder().build();

        let imp = card.imp();
        imp.vm.borrow_mut().replace(vm.clone());
        imp.app_state.borrow_mut().replace(app_state);
        if let Some(monitor) = process_monitor {
            imp.process_monitor.borrow_mut().replace(monitor);
        }

        // Update display asynchronously
        let vm_clone = vm.clone();
        let card_weak = card.downgrade();
        glib::spawn_future_local(async move {
            if let Some(card) = card_weak.upgrade() {
                let _ = card.update_display(&vm_clone).await;
            }
        });

        card
    }

    pub fn widget(&self) -> &Self {
        self
    }

    async fn update_display(&self, vm: &VM) -> Result<(), anyhow::Error> {
        let imp = self.imp();

        // Update VM name and OS
        imp.vm_name.set_text(&vm.name);
        imp.vm_os.set_text(&vm.config.guest_os);

        // Update status
        let (status_text, icon_name, status_color) = match &vm.status {
            VMStatus::Running { pid: _ } => {
                // Record start time if just started
                if imp.start_time.borrow().is_none() {
                    imp.start_time.borrow_mut().replace(std::time::Instant::now());
                }
                ("Running", "media-playback-start-symbolic", "success")
            },
            VMStatus::Stopped => {
                // Clear start time
                imp.start_time.borrow_mut().take();
                ("Stopped", "media-playback-stop-symbolic", "dim-label")
            },
            VMStatus::Starting => ("Starting", "media-playback-start-symbolic", "warning"),
            VMStatus::Stopping => ("Stopping", "media-playback-stop-symbolic", "warning"),
            VMStatus::Error(_) => ("Error", "dialog-error-symbolic", "error"),
        };

        imp.status_label.set_text(status_text);
        imp.status_icon.set_icon_name(Some(icon_name));
        
        // Clear old CSS classes and add new one
        imp.status_label.remove_css_class("success");
        imp.status_label.remove_css_class("warning");
        imp.status_label.remove_css_class("error");
        imp.status_label.add_css_class(status_color);
        
        // Update metrics if VM is running
        if let VMStatus::Running { .. } = &vm.status {
            self.update_metrics().await;
        } else {
            // Clear metrics when stopped
            let cpu_value = imp.cpu_value.clone();
            let ram_value = imp.ram_value.clone();
            let disk_value = imp.disk_value.clone();
            
            glib::idle_add_local(move || {
                cpu_value.set_text("--");
                ram_value.set_text("--");
                disk_value.set_text("--");
                // Don't hide the metrics box - just show dashes
                glib::ControlFlow::Break
            });
        }
        
        // Update control buttons based on status
        match &vm.status {
            VMStatus::Stopped => {
                // Show start button instead of status label
                imp.status_icon.set_visible(false);  // Hide the icon for stopped VMs
                imp.status_button.set_visible(true);
                imp.status_button.set_label("Start");
                imp.status_button.set_sensitive(true);
                imp.status_label.set_visible(false);
                imp.control_area.set_visible(false);
            },
            VMStatus::Running { .. } => {
                // Show console button for running VMs
                imp.status_icon.set_visible(false);  // Hide icon for running VMs
                imp.status_button.set_visible(false);
                imp.console_button.set_visible(true);
                imp.status_label.set_visible(false);
                imp.control_area.set_visible(true);
                imp.stop_button.set_sensitive(true);
                imp.stop_button.set_label("Stop VM");
            },
            VMStatus::Starting => {
                // Show disabled start button while starting
                imp.status_icon.set_visible(true);  // Show icon while starting
                imp.status_button.set_visible(true);
                imp.status_button.set_label("Starting...");
                imp.status_button.set_sensitive(false);
                imp.console_button.set_visible(false);
                imp.status_label.set_visible(false);
                imp.control_area.set_visible(false);
            },
            VMStatus::Stopping => {
                // Show status label while stopping
                imp.status_icon.set_visible(true);  // Show icon while stopping
                imp.status_button.set_visible(false);
                imp.console_button.set_visible(false);
                imp.status_label.set_visible(true);
                imp.control_area.set_visible(true);
                imp.stop_button.set_label("Stopping...");
                imp.stop_button.set_sensitive(false);
            },
            VMStatus::Error(_) => {
                // Show retry button on error
                imp.status_icon.set_visible(true);  // Show icon for error state
                imp.status_button.set_visible(true);
                imp.status_button.set_label("Retry");
                imp.status_button.set_sensitive(true);
                imp.console_button.set_visible(false);
                imp.status_label.set_visible(false);
                imp.control_area.set_visible(false);
            }
        }
        
        Ok(())
    }
    
    async fn update_metrics(&self) {
        let imp = self.imp();
        
        if let Some(monitor) = imp.process_monitor.borrow().as_ref() {
            if let Some(vm) = imp.vm.borrow().as_ref() {
                // Update metrics first
                monitor.update_metrics().await;
                
                if let Some(metrics) = monitor.get_vm_metrics(&vm.id).await {
                    // Update CPU
                    let cpu_text = format!("{:.0}%", metrics.cpu_percent);
                    
                    // Update RAM with better formatting
                    let ram_text = if metrics.memory_mb > 1024 {
                        format!("{:.1}G", metrics.memory_mb as f32 / 1024.0)
                    } else {
                        format!("{}M", metrics.memory_mb)
                    };
                    
                    // Update disk I/O
                    let disk_text = format_io_rate(metrics.disk_read_bytes + metrics.disk_write_bytes);
                    
                    // Clone the widgets for use in the main thread closure
                    let cpu_value = imp.cpu_value.clone();
                    let ram_value = imp.ram_value.clone();
                    let disk_value = imp.disk_value.clone();
                    let metrics_box = imp.metrics_box.clone();
                    
                    // Update UI on the main thread
                    glib::idle_add_local(move || {
                        cpu_value.set_text(&cpu_text);
                        ram_value.set_text(&ram_text);
                        disk_value.set_text(&disk_text);
                        metrics_box.set_visible(true);
                        
                        // Remove any problematic style classes and add working ones
                        cpu_value.remove_css_class("title-2");
                        ram_value.remove_css_class("title-2");
                        disk_value.remove_css_class("title-2");
                        
                        // Add a simple style class
                        cpu_value.add_css_class("metric-value");
                        ram_value.add_css_class("metric-value");
                        disk_value.add_css_class("metric-value");
                        
                        // Force queue redraw on the labels and box
                        cpu_value.queue_draw();
                        ram_value.queue_draw();
                        disk_value.queue_draw();
                        metrics_box.queue_draw();
                        
                        glib::ControlFlow::Break
                    });
                }
            }
        }
    }

    pub async fn refresh_status(&self, app_state: &AppState) {
        let imp = self.imp();
        
        // Extract data we need before the borrow
        let (vm_data, should_register) = {
            let mut vm_borrow = imp.vm.borrow_mut();
            if let Some(vm) = vm_borrow.as_mut() {
                let old_status = vm.status.clone();

                // Update VM status from running processes
                app_state.vm_manager.update_vm_status(vm).await;

                let status_changed = std::mem::discriminant(&old_status) != std::mem::discriminant(&vm.status);
                let vm_clone = vm.clone();
                let register_info = if status_changed {
                    match (&old_status, &vm.status) {
                        (VMStatus::Stopped, VMStatus::Running { pid }) |
                        (VMStatus::Starting, VMStatus::Running { pid }) => {
                            println!("VM {} started: Running with PID {}", vm.name, pid);
                            Some((vm.id.clone(), *pid, true))
                        }
                        (VMStatus::Running { .. }, VMStatus::Stopped) => {
                            println!("VM {} stopped", vm.name);
                            Some((vm.id.clone(), 0, false))
                        }
                        (VMStatus::Stopping, VMStatus::Stopped) => {
                            println!("VM {} stopped", vm.name);
                            None
                        }
                        _ => {
                            println!(
                                "VM {} status changed: {:?} -> {:?}",
                                vm.name, old_status, vm.status
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                
                (Some((vm_clone, status_changed)), register_info)
            } else {
                (None, None)
            }
        };
        
        // Handle process monitor registration outside of the borrow
        if let Some((vm_id, pid, should_register)) = should_register {
            if let Some(monitor) = imp.process_monitor.borrow().as_ref() {
                let monitor_clone = monitor.clone();
                if should_register {
                    glib::spawn_future_local(async move {
                        monitor_clone.register_vm_process(vm_id, pid).await;
                    });
                } else {
                    glib::spawn_future_local(async move {
                        monitor_clone.unregister_vm_process(&vm_id).await;
                    });
                }
            }
        }
        
        // Update display if status changed
        if let Some((vm_clone, status_changed)) = vm_data {
            if status_changed {
                let vm_for_display = vm_clone.clone();
                glib::spawn_future_local(
                    glib::clone!(@weak self as obj => async move {
                        let _ = obj.update_display(&vm_for_display).await;
                    }),
                );
            }
            
            // Update metrics regardless of status change
            if matches!(vm_clone.status, VMStatus::Running { .. }) {
                let obj = self.clone();
                glib::spawn_future_local(async move {
                    obj.update_metrics().await;
                });
            }
        }
    }
}

// Helper function for formatting I/O rates
fn format_io_rate(bytes: u64) -> String {
    if bytes == 0 {
        return "--".to_string();
    }
    
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
    let mut rate = bytes as f64;
    let mut unit_idx = 0;
    
    while rate >= 1024.0 && unit_idx < UNITS.len() - 1 {
        rate /= 1024.0;
        unit_idx += 1;
    }
    
    if unit_idx == 0 {
        format!("{:.0}{}", rate, UNITS[unit_idx])
    } else {
        format!("{:.1}{}", rate, UNITS[unit_idx])
    }
}
