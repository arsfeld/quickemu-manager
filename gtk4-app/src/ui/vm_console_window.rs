use crate::ui::SpiceDisplay;
use adw::prelude::*;
use gtk::{gio, glib};

pub struct VMConsoleWindow {
    window: adw::ApplicationWindow,
    spice_display: SpiceDisplay,
    vm_name: String,
}

impl VMConsoleWindow {
    pub fn new(app: &gtk::Application, vm_name: String) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(format!("{} - Console", vm_name))
            .default_width(800)
            .default_height(600)
            .build();

        // Create header bar with controls
        let header_bar = adw::HeaderBar::new();

        // Add fullscreen button
        let fullscreen_button = gtk::Button::builder()
            .icon_name("view-fullscreen-symbolic")
            .tooltip_text("Toggle Fullscreen")
            .build();

        fullscreen_button.connect_clicked(glib::clone!(
            #[weak]
            window,
            move |_| {
                if window.is_fullscreen() {
                    window.unfullscreen();
                } else {
                    window.fullscreen();
                }
            }
        ));

        header_bar.pack_end(&fullscreen_button);

        // Create toolbar with VM controls
        let toolbar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        // Send Ctrl+Alt+Del button
        let cad_button = gtk::Button::builder()
            .label("Ctrl+Alt+Del")
            .tooltip_text("Send Ctrl+Alt+Del to VM")
            .build();
        toolbar.append(&cad_button);

        // Create main content box
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        content_box.append(&header_bar);
        content_box.append(&toolbar);

        // Create SPICE display widget
        let spice_display = SpiceDisplay::new();
        spice_display.set_vexpand(true);
        spice_display.set_hexpand(true);
        spice_display.set_visible(true);

        // Set a background color to see if the widget is there
        spice_display.add_css_class("spice-display-container");

        // Wrap in a scrolled window for better handling of different resolutions
        let scrolled_window = gtk::ScrolledWindow::builder()
            .child(&spice_display)
            .vexpand(true)
            .hexpand(true)
            .build();

        content_box.append(&scrolled_window);

        window.set_content(Some(&content_box));

        // Handle window close
        window.connect_close_request(glib::clone!(
            #[weak]
            spice_display,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                spice_display.disconnect();
                glib::Propagation::Proceed
            }
        ));

        Self {
            window,
            spice_display,
            vm_name,
        }
    }

    pub fn connect_to_vm(&self, host: &str, port: u16) {
        self.spice_display.connect(host.to_string(), port);
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn close(&self) {
        self.window.close();
    }
}
