use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, TemplateChild};
use std::cell::RefCell;
use std::rc::Rc;

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
        pub uptime_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub connect_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub edit_button: TemplateChild<gtk::Button>,

        pub vm: Rc<RefCell<Option<VM>>>,
        pub app_state: Rc<RefCell<Option<AppState>>>,
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
        fn start_vm_static(vm: VM, app_state: AppState) {
            glib::spawn_future_local(async move {
                if let Err(e) = app_state.vm_manager.start_vm(&vm).await {
                    eprintln!("Failed to start VM: {}", e);
                }
                // Note: Status will be refreshed by the periodic refresh timer
            });
        }

        fn stop_vm_static(vm: VM, app_state: AppState) {
            glib::spawn_future_local(async move {
                if let Err(e) = app_state.vm_manager.stop_vm(&vm.id).await {
                    eprintln!("Failed to stop VM: {}", e);
                }
                // Note: Status will be refreshed by the periodic refresh timer
            });
        }

        fn launch_console_static(vm: VM, button: gtk::Button) {
            // Find the parent main window
            if let Some(window) = button.root() {
                if let Some(main_window) = window.downcast_ref::<MainWindow>() {
                    // Use the main window's console view
                    main_window.show_vm_console(vm);
                }
            }
        }
    }

    impl ObjectImpl for VMCard {
        fn constructed(&self) {
            self.parent_constructed();

            // Connect signals manually
            let start_button = self.start_button.get();
            let vm = self.vm.clone();
            let app_state = self.app_state.clone();
            start_button.connect_clicked(move |_| {
                let vm_ref = vm.borrow();
                let app_state_ref = app_state.borrow();

                if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                    Self::start_vm_static(vm.clone(), app_state.clone());
                }
            });

            let stop_button = self.stop_button.get();
            let vm = self.vm.clone();
            let app_state = self.app_state.clone();
            stop_button.connect_clicked(move |_| {
                let vm_ref = vm.borrow();
                let app_state_ref = app_state.borrow();

                if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                    Self::stop_vm_static(vm.clone(), app_state.clone());
                }
            });

            let connect_button = self.connect_button.get();
            let vm = self.vm.clone();
            connect_button.connect_clicked(move |button| {
                let vm_ref = vm.borrow();

                if let Some(vm) = vm_ref.as_ref() {
                    Self::launch_console_static(vm.clone(), button.clone());
                }
            });

            let edit_button = self.edit_button.get();
            let vm = self.vm.clone();
            let app_state = self.app_state.clone();
            edit_button.connect_clicked(move |button| {
                let vm_ref = vm.borrow();
                let app_state_ref = app_state.borrow();

                if let (Some(vm), Some(app_state)) = (vm_ref.as_ref(), app_state_ref.as_ref()) {
                    // Find the parent window
                    if let Some(window) = button
                        .root()
                        .and_then(|root| root.downcast::<gtk::Window>().ok())
                    {
                        let dialog = VMEditDialog::new(&window, vm.clone(), app_state.clone());
                        dialog.present();
                    }
                }
            });
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
                uptime_value: TemplateChild::default(),
                start_button: TemplateChild::default(),
                stop_button: TemplateChild::default(),
                connect_button: TemplateChild::default(),
                edit_button: TemplateChild::default(),
                vm: Rc::new(RefCell::new(None)),
                app_state: Rc::new(RefCell::new(None)),
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
    pub fn new(vm: VM, app_state: AppState) -> Self {
        let card: Self = glib::Object::builder().build();

        let imp = card.imp();
        imp.vm.borrow_mut().replace(vm.clone());
        imp.app_state.borrow_mut().replace(app_state);

        card.update_display(&vm);

        card
    }

    pub fn widget(&self) -> &Self {
        self
    }

    fn update_display(&self, vm: &VM) {
        let imp = self.imp();

        // Update VM name and OS
        imp.vm_name.set_text(&vm.name);
        imp.vm_os.set_text(&vm.config.guest_os);

        // Update status
        let (status_text, icon_name) = match &vm.status {
            VMStatus::Running { pid: _ } => ("Running", "media-playback-start-symbolic"),
            VMStatus::Stopped => ("Stopped", "media-playback-stop-symbolic"),
            VMStatus::Starting => ("Starting", "media-playback-start-symbolic"),
            VMStatus::Stopping => ("Stopping", "media-playback-stop-symbolic"),
            VMStatus::Error(_) => ("Error", "dialog-error-symbolic"),
        };

        imp.status_label.set_text(status_text);
        imp.status_icon.set_icon_name(Some(icon_name));

        // Update resource values (placeholder for now)
        imp.cpu_value.set_text("0%");
        imp.ram_value.set_text("0 MB");

        if let VMStatus::Running { pid: _ } = &vm.status {
            imp.uptime_value.set_text("Running");
        } else {
            imp.uptime_value.set_text("--");
        }

        // Update button states
        match &vm.status {
            VMStatus::Stopped => {
                imp.start_button.set_sensitive(true);
                imp.stop_button.set_sensitive(false);
                imp.connect_button.set_sensitive(false);
                imp.edit_button.set_sensitive(true);
            }
            VMStatus::Running { pid: _ } => {
                imp.start_button.set_sensitive(false);
                imp.stop_button.set_sensitive(true);
                imp.connect_button.set_sensitive(true);
                imp.edit_button.set_sensitive(false); // Can't edit while running
            }
            _ => {
                imp.start_button.set_sensitive(false);
                imp.stop_button.set_sensitive(false);
                imp.connect_button.set_sensitive(false);
                imp.edit_button.set_sensitive(false);
            }
        }
    }

    pub async fn refresh_status(&self, app_state: &AppState) {
        let imp = self.imp();
        if let Some(vm) = imp.vm.borrow_mut().as_mut() {
            let old_status = vm.status.clone();

            // Update VM status from running processes
            app_state.vm_manager.update_vm_status(vm).await;

            // Only print status changes
            if std::mem::discriminant(&old_status) != std::mem::discriminant(&vm.status) {
                match (&old_status, &vm.status) {
                    (VMStatus::Stopped, VMStatus::Running { pid }) => {
                        println!("VM {} started: Running with PID {}", vm.name, pid);
                    }
                    (VMStatus::Running { .. }, VMStatus::Stopped) => {
                        println!("VM {} stopped", vm.name);
                    }
                    (VMStatus::Starting, VMStatus::Running { pid }) => {
                        println!("VM {} started: Running with PID {}", vm.name, pid);
                    }
                    (VMStatus::Stopping, VMStatus::Stopped) => {
                        println!("VM {} stopped", vm.name);
                    }
                    _ => {
                        println!(
                            "VM {} status changed: {:?} -> {:?}",
                            vm.name, old_status, vm.status
                        );
                    }
                }

                // Clone the VM to update the display when status changes
                let vm_clone = vm.clone();
                glib::idle_add_local(
                    glib::clone!(@weak self as obj => @default-return glib::ControlFlow::Break, move || {
                        obj.update_display(&vm_clone);
                        glib::ControlFlow::Break
                    }),
                );
            }
        }
    }
}
