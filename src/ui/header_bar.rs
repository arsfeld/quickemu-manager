use gtk::prelude::*;
use gtk::{glib, gio, CompositeTemplate, TemplateChild};
use adw::subclass::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/quickemu/Manager/ui/header_bar.ui")]
    pub struct HeaderBar {
        #[template_child]
        pub create_vm_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub refresh_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        
        pub create_vm_callbacks: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
        pub settings_callbacks: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
        pub refresh_callbacks: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HeaderBar {
        const NAME: &'static str = "QEMHeaderBar";
        type Type = super::HeaderBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }


    impl ObjectImpl for HeaderBar {
        fn constructed(&self) {
            self.parent_constructed();
            
            // Connect signals manually
            let obj = self.obj();
            
            let create_vm_button = self.create_vm_button.get();
            let callbacks = self.create_vm_callbacks.clone();
            create_vm_button.connect_clicked(move |_| {
                let callbacks = callbacks.borrow();
                for callback in callbacks.iter() {
                    callback();
                }
            });
            
            let refresh_button = self.refresh_button.get();
            let callbacks = self.refresh_callbacks.clone();
            refresh_button.connect_clicked(move |_| {
                let callbacks = callbacks.borrow();
                for callback in callbacks.iter() {
                    callback();
                }
            });
        }
    }

    impl WidgetImpl for HeaderBar {}
    impl BoxImpl for HeaderBar {}

    impl Default for HeaderBar {
        fn default() -> Self {
            Self {
                create_vm_button: TemplateChild::default(),
                refresh_button: TemplateChild::default(),
                menu_button: TemplateChild::default(),
                create_vm_callbacks: Rc::new(RefCell::new(Vec::new())),
                settings_callbacks: Rc::new(RefCell::new(Vec::new())),
                refresh_callbacks: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }
}

glib::wrapper! {
    pub struct HeaderBar(ObjectSubclass<imp::HeaderBar>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl HeaderBar {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn widget(&self) -> &Self {
        self
    }

    pub fn connect_create_vm<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        self.imp().create_vm_callbacks.borrow_mut().push(Box::new(f));
    }

    pub fn connect_settings<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        self.imp().settings_callbacks.borrow_mut().push(Box::new(f));
    }

    pub fn connect_refresh<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        self.imp().refresh_callbacks.borrow_mut().push(Box::new(f));
    }
}