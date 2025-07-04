use gtk::prelude::*;
use gtk::{glib, gio, CompositeTemplate, TemplateChild};
use adw::subclass::prelude::*;
use adw::prelude::AdwApplicationWindowExt;

use crate::AppState;
use quickemu_core::VM;
use super::{VMCard, VMCreateDialog, SettingsDialog, SpiceDisplay};

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/quickemu/Manager/ui/main_window.ui")]
    pub struct MainWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        pub create_vm_button: RefCell<Option<gtk::Button>>,
        pub refresh_button: RefCell<Option<gtk::Button>>,
        pub menu_button: RefCell<Option<gtk::MenuButton>>,
        #[template_child]
        pub vms_container: TemplateChild<gtk::Box>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub view_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub console_container: TemplateChild<gtk::Box>,
        
        pub app_state: RefCell<Option<AppState>>,
        pub runtime: RefCell<Option<tokio::runtime::Runtime>>,
        pub current_vm: RefCell<Option<quickemu_core::VM>>,
        pub back_button: RefCell<Option<gtk::Button>>,
        pub console_widget: RefCell<Option<SpiceDisplay>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        const NAME: &'static str = "QEMMainWindow";
        type Type = super::MainWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MainWindow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MainWindow {}
    impl WindowImpl for MainWindow {}
    impl ApplicationWindowImpl for MainWindow {}
    impl AdwApplicationWindowImpl for MainWindow {}

    impl Default for MainWindow {
        fn default() -> Self {
            Self {
                header_bar: TemplateChild::default(),
                create_vm_button: RefCell::new(None),
                refresh_button: RefCell::new(None),
                menu_button: RefCell::new(None),
                vms_container: TemplateChild::default(),
                scrolled_window: TemplateChild::default(),
                toast_overlay: TemplateChild::default(),
                view_stack: TemplateChild::default(),
                console_container: TemplateChild::default(),
                app_state: RefCell::new(None),
                runtime: RefCell::new(None),
                current_vm: RefCell::new(None),
                back_button: RefCell::new(None),
                console_widget: RefCell::new(None),
            }
        }
    }
}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MainWindow {
    pub fn new(app: &gtk::Application, app_state: AppState, runtime: tokio::runtime::Runtime) -> Self {
        println!("Creating MainWindow...");
        let window: Self = glib::Object::builder()
            .property("application", app)
            .build();
        println!("MainWindow created successfully");

        let imp = window.imp();
        imp.app_state.replace(Some(app_state.clone()));
        imp.runtime.replace(Some(runtime));
        
        // Set up view stack transitions
        let view_stack = &*imp.view_stack;
        // Note: AdwViewStack doesn't have programmable transitions like GtkStack
        // The transitions are controlled by the ViewSwitcher when used
        
        // Use the header bar from the template
        let header_bar = &*imp.header_bar;
        
        // Create title label
        let title_label = gtk::Label::new(Some("Quickemu Manager"));
        title_label.add_css_class("title");
        header_bar.set_title_widget(Some(&title_label));
        
        // Create back button (initially hidden)
        let back_button = gtk::Button::new();
        back_button.set_icon_name("go-previous-symbolic");
        back_button.set_tooltip_text(Some("Back to VM List"));
        back_button.set_visible(false);
        header_bar.pack_start(&back_button);
        
        // Create refresh button
        let refresh_button = gtk::Button::new();
        refresh_button.set_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh VM List"));
        header_bar.pack_start(&refresh_button);
        
        // Create create VM button
        let create_vm_button = gtk::Button::new();
        create_vm_button.set_icon_name("list-add-symbolic");
        create_vm_button.set_tooltip_text(Some("Create New VM"));
        create_vm_button.add_css_class("suggested-action");
        header_bar.pack_end(&create_vm_button);
        
        // Create menu button
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Main Menu"));
        
        // Create menu model programmatically
        let menu = gio::Menu::new();
        let section = gio::Menu::new();
        section.append(Some("Settings"), Some("win.settings"));
        section.append(Some("About"), Some("win.about"));
        menu.append_section(None, &section);
        
        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        menu_button.set_popover(Some(&popover));
        header_bar.pack_end(&menu_button);
        
        // Store references to buttons
        imp.create_vm_button.replace(Some(create_vm_button.clone()));
        imp.refresh_button.replace(Some(refresh_button.clone()));
        imp.menu_button.replace(Some(menu_button));
        imp.back_button.replace(Some(back_button.clone()));

        // Connect button signals
        let window_weak = window.downgrade();
        let app_state_clone = app_state.clone();
        create_vm_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                let dialog = VMCreateDialog::new(&window, app_state_clone.clone());
                dialog.present();
            }
        });

        let window_weak = window.downgrade();
        refresh_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.refresh_vms();
            }
        });
        
        // Connect back button
        let window_weak = window.downgrade();
        back_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.show_vm_list();
            }
        });

        // Set up menu actions
        let settings_action = gio::ActionEntry::builder("settings")
            .activate(glib::clone!(@weak window, @strong app_state => move |_, _, _| {
                let dialog = SettingsDialog::new(&window, app_state.clone());
                dialog.present();
            }))
            .build();

        let about_action = gio::ActionEntry::builder("about")
            .activate(move |_, _, _| {
                // TODO: Implement about dialog
                println!("About dialog not yet implemented");
            })
            .build();

        window.add_action_entries([settings_action, about_action]);

        // Load VMs
        window.load_vms();
        
        // Set up periodic refresh
        window.setup_periodic_refresh();

        window
    }

    pub fn present(&self) {
        self.set_visible(true);
    }
    
    pub fn refresh_vms(&self) {
        self.load_vms();
    }
    
    pub fn show_vm_list(&self) {
        let imp = self.imp();
        imp.view_stack.set_visible_child_name("vm_list");
        
        // Hide back button and show refresh button
        if let Some(back_button) = imp.back_button.borrow().as_ref() {
            back_button.set_visible(false);
        }
        if let Some(refresh_button) = imp.refresh_button.borrow().as_ref() {
            refresh_button.set_visible(true);
        }
        
        // Clean up console if it exists
        if let Some(console_widget) = imp.console_widget.borrow_mut().take() {
            imp.console_container.remove(&console_widget);
        }
        
        // Clear current VM
        imp.current_vm.borrow_mut().take();
    }
    
    pub fn show_vm_console(&self, vm: quickemu_core::VM) {
        let imp = self.imp();
        
        // Store current VM
        imp.current_vm.borrow_mut().replace(vm.clone());
        
        // Get SPICE port from VM configuration
        let spice_port = match &vm.config.display {
            quickemu_core::DisplayProtocol::Spice { port } => *port,
            _ => {
                eprintln!("VM {} is not configured for SPICE display", vm.name);
                return;
            }
        };
        
        // Create console widget
        let console_widget = SpiceDisplay::new();
        imp.console_container.append(&console_widget);
        imp.console_widget.borrow_mut().replace(console_widget.clone());
        
        // Connect to VM
        console_widget.connect("localhost".to_string(), spice_port);
        
        // Switch to console view
        imp.view_stack.set_visible_child_name("vm_console");
        
        // Show back button and hide refresh button
        if let Some(back_button) = imp.back_button.borrow().as_ref() {
            back_button.set_visible(true);
        }
        if let Some(refresh_button) = imp.refresh_button.borrow().as_ref() {
            refresh_button.set_visible(false);
        }
    }

    fn load_vms(&self) {
        let imp = self.imp();
        let vms_container = &imp.vms_container;
        
        // Clear existing VMs
        while let Some(child) = vms_container.first_child() {
            vms_container.remove(&child);
        }

        // Try to discover VMs asynchronously
        let app_state = imp.app_state.borrow().clone();
        
        if let Some(app_state) = app_state {
            glib::spawn_future_local(glib::clone!(@weak vms_container => async move {
                let mut all_vms = Vec::new();
                let vm_directories = app_state.config_manager.get_all_vm_directories().await;
                
                for dir in &vm_directories {
                    if let Ok(vms) = MainWindow::discover_vms_in_directory(&app_state, dir).await {
                        all_vms.extend(vms);
                    }
                }
                
                if all_vms.is_empty() {
                    MainWindow::show_empty_state(&vms_container);
                } else {
                    MainWindow::show_vm_grid(&vms_container, all_vms, app_state);
                }
            }));
        } else {
            Self::show_empty_state(vms_container);
        }
    }

    async fn discover_vms_in_directory(
        app_state: &AppState,
        dir: &std::path::Path,
    ) -> Result<Vec<VM>, anyhow::Error> {
        use quickemu_core::{VMDiscovery, DiscoveryEvent};
        use tokio::sync::mpsc;
        
        let (event_tx, _) = mpsc::unbounded_channel::<DiscoveryEvent>();
        let mut discovery = VMDiscovery::with_vm_manager(event_tx, app_state.vm_manager.clone());
        discovery.scan_directory(dir).await
    }

    fn show_empty_state(container: &gtk::Box) {
        let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        empty_box.set_halign(gtk::Align::Center);
        empty_box.set_valign(gtk::Align::Center);
        empty_box.set_vexpand(true);

        let icon = gtk::Image::from_icon_name("folder-symbolic");
        icon.set_pixel_size(64);
        icon.add_css_class("dim-label");

        let title = gtk::Label::new(Some("No Virtual Machines Found"));
        title.set_markup("<span size='large' weight='bold'>No Virtual Machines Found</span>");
        title.add_css_class("dim-label");

        let subtitle = gtk::Label::new(Some("Click 'Create VM' to get started"));
        subtitle.add_css_class("dim-label");

        empty_box.append(&icon);
        empty_box.append(&title);
        empty_box.append(&subtitle);

        container.append(&empty_box);
    }

    fn show_vm_grid(container: &gtk::Box, vms: Vec<VM>, app_state: AppState) {
        // Create a flow box for responsive grid layout
        let flowbox = gtk::FlowBox::new();
        flowbox.set_max_children_per_line(3);
        flowbox.set_min_children_per_line(1);
        flowbox.set_row_spacing(12);
        flowbox.set_column_spacing(12);
        flowbox.set_homogeneous(true);
        flowbox.set_selection_mode(gtk::SelectionMode::None);

        for vm in vms {
            let vm_card = VMCard::new(vm.clone(), app_state.clone());
            flowbox.append(vm_card.widget());
        }

        container.append(&flowbox);
    }
    
    fn setup_periodic_refresh(&self) {
        let window_weak = self.downgrade();
        let imp = self.imp();
        let app_state = imp.app_state.borrow().clone();
        
        glib::timeout_add_seconds_local(1, move || {
            if let Some(window) = window_weak.upgrade() {
                if let Some(app_state) = &app_state {
                    window.update_vm_statuses(app_state.clone());
                }
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }
    
    fn update_vm_statuses(&self, app_state: AppState) {
        let imp = self.imp();
        let vms_container = &imp.vms_container;
        
        // Find the flow box containing VM cards
        if let Some(flowbox) = vms_container.first_child() {
            if let Some(flowbox) = flowbox.downcast_ref::<gtk::FlowBox>() {
                let mut child = flowbox.first_child();
                
                while let Some(widget) = child {
                    if let Some(vm_card) = widget.first_child() {
                        if let Some(vm_card) = vm_card.downcast_ref::<VMCard>() {
                            glib::spawn_future_local(glib::clone!(@weak vm_card, @strong app_state => async move {
                                vm_card.refresh_status(&app_state).await;
                            }));
                        }
                    }
                    child = widget.next_sibling();
                }
            }
        }
    }
}