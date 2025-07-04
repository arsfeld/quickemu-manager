use adw::subclass::prelude::*;
use adw::{prelude::*, ComboRow, SwitchRow};
use gtk::prelude::*;
use gtk::{glib, Button};

use crate::AppState;
use quickemu_core::Theme;

mod imp {
    use super::*;
    use gtk::{CompositeTemplate, TemplateChild};
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/quickemu/Manager/ui/settings_dialog.ui")]
    pub struct SettingsDialog {
        #[template_child]
        pub close_button: TemplateChild<Button>,
        #[template_child]
        pub add_dir_button: TemplateChild<Button>,
        #[template_child]
        pub auto_download_switch: TemplateChild<SwitchRow>,
        #[template_child]
        pub theme_row: TemplateChild<ComboRow>,

        pub app_state: RefCell<Option<AppState>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsDialog {
        const NAME: &'static str = "QEMSettingsDialog";
        type Type = super::SettingsDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SettingsDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SettingsDialog {}
    impl WindowImpl for SettingsDialog {}
    impl AdwWindowImpl for SettingsDialog {}

    impl Default for SettingsDialog {
        fn default() -> Self {
            Self {
                close_button: TemplateChild::default(),
                add_dir_button: TemplateChild::default(),
                auto_download_switch: TemplateChild::default(),
                theme_row: TemplateChild::default(),
                app_state: RefCell::new(None),
            }
        }
    }
}

glib::wrapper! {
    pub struct SettingsDialog(ObjectSubclass<imp::SettingsDialog>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SettingsDialog {
    pub fn new(parent: &impl IsA<gtk::Window>, app_state: AppState) -> Self {
        let dialog: Self = glib::Object::builder()
            .property("transient-for", parent)
            .build();

        let imp = dialog.imp();
        imp.app_state.replace(Some(app_state.clone()));

        // Connect close button
        let dialog_weak = dialog.downgrade();
        imp.close_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });

        // Connect settings changes
        let app_state_clone = app_state.clone();
        imp.auto_download_switch
            .connect_active_notify(move |switch| {
                let app_state = app_state_clone.clone();
                let active = switch.is_active();
                glib::spawn_future_local(async move {
                    if let Err(e) = app_state
                        .config_manager
                        .update_config(|config| {
                            config.auto_download_tools = active;
                        })
                        .await
                    {
                        eprintln!("Failed to save config: {}", e);
                    }
                });
            });

        let app_state_clone = app_state.clone();
        imp.theme_row.connect_selected_notify(move |combo| {
            let app_state = app_state_clone.clone();
            let selected = combo.selected();
            glib::spawn_future_local(async move {
                if let Err(e) = app_state
                    .config_manager
                    .update_config(|config| {
                        config.theme = match selected {
                            1 => Theme::Light,
                            2 => Theme::Dark,
                            _ => Theme::System,
                        };
                    })
                    .await
                {
                    eprintln!("Failed to save config: {}", e);
                }
            });
        });

        // Load current settings
        let auto_download_switch = imp.auto_download_switch.clone();
        let theme_row = imp.theme_row.clone();
        glib::spawn_future_local(async move {
            let config = app_state.config_manager.get_config().await;
            auto_download_switch.set_active(config.auto_download_tools);

            let theme_index = match config.theme {
                Theme::System => 0,
                Theme::Light => 1,
                Theme::Dark => 2,
            };
            theme_row.set_selected(theme_index);
        });

        dialog
    }

    pub fn present(&self) {
        self.set_visible(true);
    }
}
