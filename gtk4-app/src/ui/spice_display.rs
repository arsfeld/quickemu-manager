use glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{cairo, gdk, glib};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

// For native builds, we need to use SpiceClientShared
#[cfg(not(target_arch = "wasm32"))]
use spice_client::SpiceClientShared;

glib::wrapper! {
    pub struct SpiceDisplay(ObjectSubclass<imp::SpiceDisplay>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SpiceDisplay {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn connect(&self, host: String, port: u16) {
        eprintln!("SpiceDisplay: Starting connection to {}:{}", host, port);

        // Update placeholder text
        self.imp().update_status("Connecting...");

        // Create a shared tokio runtime that we'll use throughout
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => Arc::new(rt),
            Err(e) => {
                eprintln!("Failed to create tokio runtime: {}", e);
                self.imp().update_status(&format!("Runtime error: {}", e));
                return;
            }
        };

        // Store runtime in implementation
        self.imp().store_runtime(runtime.clone());

        let widget_weak = self.downgrade();
        let host_clone = host.clone();
        let port_clone = port;

        // Use glib's spawn_future_local to run async code in the main thread
        glib::spawn_future_local(async move {
            eprintln!("SpiceDisplay: Connecting to {}:{}", host_clone, port_clone);

            // Create the client in a tokio context
            let client = runtime
                .spawn(async move { SpiceClientShared::new(host_clone, port_clone) })
                .await
                .unwrap();

            // Connect to the server
            eprintln!("SpiceDisplay: Calling client.connect()...");
            let client_for_connect = client.clone();
            let connect_result = runtime
                .spawn(async move { client_for_connect.connect().await })
                .await
                .unwrap();

            match connect_result {
                Ok(_) => {
                    eprintln!("SpiceDisplay: Connected to SPICE server successfully");
                }
                Err(e) => {
                    eprintln!("Failed to connect to SPICE server: {}", e);
                    if let Some(widget) = widget_weak.upgrade() {
                        widget
                            .imp()
                            .update_status(&format!("Connection failed: {}", e));
                    }
                    return;
                }
            }

            // Start the event loop
            eprintln!("SpiceDisplay: Starting event loop...");
            let client_clone = client.clone();
            let event_loop_result = runtime
                .spawn(async move { client_clone.start_event_loop().await })
                .await
                .unwrap();

            match event_loop_result {
                Ok(_) => {
                    eprintln!("SpiceDisplay: Event loop started successfully");
                }
                Err(e) => {
                    eprintln!("Failed to start SPICE event loop: {}", e);
                    if let Some(widget) = widget_weak.upgrade() {
                        widget
                            .imp()
                            .update_status(&format!("Event loop failed: {}", e));
                    }
                    return;
                }
            }

            // Now set up the display
            if let Some(widget) = widget_weak.upgrade() {
                widget.imp().update_status("Connected");
                widget.imp().setup_display_with_runtime(client, runtime);
            }
        });
    }

    pub fn disconnect(&self) {
        let imp = self.imp();
        imp.stop_display_updates();
    }
}

impl Default for SpiceDisplay {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use super::*;
    use spice_client::multimedia::{
        gtk4::{display::Gtk4Display, Gtk4Backend},
        spice_adapter::SpiceDisplayAdapter,
        MultimediaBackend,
    };
    use tokio::sync::watch;

    pub struct SpiceDisplay {
        pub display_adapter: RefCell<Option<Arc<SpiceDisplayAdapter>>>,
        pub update_cancel_tx: RefCell<Option<watch::Sender<bool>>>,
        pub client: RefCell<Option<SpiceClientShared>>,
        pub status_label: RefCell<Option<gtk::Label>>,
        pub runtime: RefCell<Option<Arc<tokio::runtime::Runtime>>>,
    }

    impl Default for SpiceDisplay {
        fn default() -> Self {
            Self {
                display_adapter: RefCell::new(None),
                update_cancel_tx: RefCell::new(None),
                client: RefCell::new(None),
                status_label: RefCell::new(None),
                runtime: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SpiceDisplay {
        const NAME: &'static str = "SpiceDisplay";
        type Type = super::SpiceDisplay;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for SpiceDisplay {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk::Orientation::Vertical);
            obj.set_visible(true);

            // Add a status label
            let status_label = gtk::Label::new(Some("Initializing SPICE display..."));
            status_label.set_visible(true);
            status_label.set_vexpand(false);
            status_label.set_margin_top(10);
            status_label.set_margin_bottom(10);
            obj.append(&status_label);

            *self.status_label.borrow_mut() = Some(status_label);
        }
    }

    impl WidgetImpl for SpiceDisplay {}
    impl BoxImpl for SpiceDisplay {}

    impl SpiceDisplay {
        pub fn update_status(&self, text: &str) {
            if let Some(label) = self.status_label.borrow().as_ref() {
                label.set_text(text);
            }
        }

        pub fn store_runtime(&self, runtime: Arc<tokio::runtime::Runtime>) {
            *self.runtime.borrow_mut() = Some(runtime);
        }

        pub fn setup_display_with_runtime(
            &self,
            client: SpiceClientShared,
            runtime: Arc<tokio::runtime::Runtime>,
        ) {
            let obj = self.obj();
            let widget_weak = obj.downgrade();

            // Store client reference
            *self.client.borrow_mut() = Some(client.clone());

            glib::spawn_future_local(async move {
                eprintln!("SpiceDisplay: Setting up display");

                // Create the GTK4 multimedia backend
                let backend = match runtime.spawn(async { Gtk4Backend::new() }).await.unwrap() {
                    Ok(backend) => backend,
                    Err(e) => {
                        eprintln!("Failed to create GTK4 backend: {}", e);
                        if let Some(widget) = widget_weak.upgrade() {
                            widget.imp().update_status(&format!("Backend error: {}", e));
                        }
                        return;
                    }
                };

                eprintln!("SpiceDisplay: GTK4 backend created");

                // Create display instance
                let mut display = match runtime
                    .spawn(async move { backend.create_display() })
                    .await
                    .unwrap()
                {
                    Ok(display) => display,
                    Err(e) => {
                        eprintln!("Failed to create display: {}", e);
                        if let Some(widget) = widget_weak.upgrade() {
                            widget.imp().update_status(&format!("Display error: {}", e));
                        }
                        return;
                    }
                };

                eprintln!("SpiceDisplay: Display instance created");

                // Get the drawing area before moving display
                let drawing_area = {
                    // Get the GTK4 display and drawing area
                    let gtk_display: &mut Gtk4Display = match display.as_any_mut().downcast_mut() {
                        Some(d) => d,
                        None => {
                            eprintln!("Failed to downcast to Gtk4Display");
                            if let Some(widget) = widget_weak.upgrade() {
                                widget.imp().update_status("Display type error");
                            }
                            return;
                        }
                    };

                    // Create display surface
                    use spice_client::multimedia::display::{Display, DisplayMode};
                    if let Err(e) = gtk_display.create_surface(DisplayMode {
                        width: 1024,
                        height: 768,
                        fullscreen: false,
                    }) {
                        eprintln!("Failed to create display surface: {}", e);
                        if let Some(widget) = widget_weak.upgrade() {
                            widget.imp().update_status(&format!("Surface error: {}", e));
                        }
                        return;
                    }

                    eprintln!("SpiceDisplay: Display surface created");

                    // Get the drawing area and clone it to avoid lifetime issues
                    gtk_display.get_drawing_area().map(|area| area.clone())
                };

                // Create the SPICE display adapter
                let adapter = Arc::new(SpiceDisplayAdapter::new(
                    client.clone(),
                    Box::new(display),
                    0, // Display channel ID
                ));

                // Now handle the drawing area if we got one
                if let Some(drawing_area) = drawing_area {
                    eprintln!("SpiceDisplay: Got drawing area, adding to widget");

                    if let Some(widget) = widget_weak.upgrade() {
                        widget.imp().add_drawing_area_and_start_updates(
                            &drawing_area,
                            adapter,
                            client,
                        );
                    }
                } else {
                    eprintln!("ERROR: No drawing area available from Gtk4Display");
                    if let Some(widget) = widget_weak.upgrade() {
                        widget.imp().update_status("No drawing area");
                    }
                }
            });
        }

        pub fn add_drawing_area_and_start_updates(
            &self,
            drawing_area: &gtk::DrawingArea,
            adapter: Arc<SpiceDisplayAdapter>,
            client: SpiceClientShared,
        ) {
            let obj = self.obj();

            eprintln!("SpiceDisplay: Adding drawing area to widget");

            // Remove the status label if present
            if let Some(label) = self.status_label.borrow_mut().take() {
                obj.remove(&label);
            }

            // Make sure the drawing area is visible and has a minimum size
            drawing_area.set_vexpand(true);
            drawing_area.set_hexpand(true);
            drawing_area.set_visible(true);
            drawing_area.set_size_request(640, 480);

            // Add a background color to confirm it's visible
            let css_provider = gtk::CssProvider::new();
            css_provider.load_from_data("drawingarea { background-color: #222; }");
            drawing_area.add_css_class("spice-drawing-area");

            obj.append(drawing_area);
            obj.set_visible(true);

            eprintln!("SpiceDisplay: Drawing area added and made visible");

            // Store the adapter
            *self.display_adapter.borrow_mut() = Some(adapter.clone());

            // Create cancellation channel
            let (cancel_tx, cancel_rx) = watch::channel(false);
            *self.update_cancel_tx.borrow_mut() = Some(cancel_tx);

            // Start display update loop
            eprintln!("SpiceDisplay: Starting update loop");
            let cancel_rx_clone = cancel_rx.clone();
            glib::spawn_future_local(async move {
                loop {
                    // Check for cancellation
                    if *cancel_rx_clone.borrow() {
                        eprintln!("SpiceDisplay: Update loop cancelled");
                        break;
                    }

                    // Wait for next frame time
                    glib::timeout_future(std::time::Duration::from_millis(16)).await;

                    // Update display
                    if let Err(e) = adapter.update_display().await {
                        eprintln!("Display update error: {}", e);
                    }
                }
            });

            // Set up input handlers
            self.setup_input_handlers(client);

            eprintln!("SpiceDisplay: Setup complete");
        }

        pub fn stop_display_updates(&self) {
            if let Some(tx) = self.update_cancel_tx.borrow_mut().take() {
                let _ = tx.send(true);
            }

            // Disconnect client if present
            if let Some(client) = self.client.borrow_mut().take() {
                glib::spawn_future_local(async move {
                    client.disconnect().await;
                });
            }
        }

        fn setup_input_handlers(&self, shared_client: SpiceClientShared) {
            use spice_client::multimedia::{
                input::{InputEvent, KeyboardEvent, MouseButton, MouseEvent},
                spice_adapter::SpiceInputAdapter,
            };

            let input_adapter = Arc::new(SpiceInputAdapter::new(shared_client, 0));
            let obj = self.obj();

            // Focus handling
            obj.set_can_focus(true);
            obj.set_focusable(true);

            // Mouse motion
            let motion_controller = gtk::EventControllerMotion::new();
            let adapter = input_adapter.clone();
            motion_controller.connect_motion(move |_, x, y| {
                let event = InputEvent::Mouse(MouseEvent::Motion {
                    x: x as u32,
                    y: y as u32,
                    relative_x: 0,
                    relative_y: 0,
                });

                let adapter = adapter.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send mouse motion: {}", e);
                    }
                });
            });
            obj.add_controller(motion_controller);

            // Mouse buttons
            let click_gesture = gtk::GestureClick::new();
            click_gesture.set_button(0); // All buttons

            let adapter_press = input_adapter.clone();
            click_gesture.connect_pressed(move |gesture, _n_press, x, y| {
                let button = match gesture.current_button() {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => return,
                };

                let event = InputEvent::Mouse(MouseEvent::Button {
                    button,
                    pressed: true,
                    x: x as u32,
                    y: y as u32,
                });

                let adapter = adapter_press.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send mouse press: {}", e);
                    }
                });
            });

            let adapter_release = input_adapter.clone();
            click_gesture.connect_released(move |gesture, _n_press, x, y| {
                let button = match gesture.current_button() {
                    1 => MouseButton::Left,
                    2 => MouseButton::Middle,
                    3 => MouseButton::Right,
                    _ => return,
                };

                let event = InputEvent::Mouse(MouseEvent::Button {
                    button,
                    pressed: false,
                    x: x as u32,
                    y: y as u32,
                });

                let adapter = adapter_release.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send mouse release: {}", e);
                    }
                });
            });
            obj.add_controller(click_gesture);

            // Keyboard
            let key_controller = gtk::EventControllerKey::new();

            let adapter_key_press = input_adapter.clone();
            key_controller.connect_key_pressed(move |_, _keyval, keycode, _state| {
                let event = InputEvent::Keyboard(KeyboardEvent::KeyDown {
                    scancode: keycode,
                    keycode: Some(keycode),
                    modifiers: 0, // TODO: Convert GTK modifiers
                });

                let adapter = adapter_key_press.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send key press: {}", e);
                    }
                });

                glib::Propagation::Stop
            });

            let adapter_key_release = input_adapter.clone();
            key_controller.connect_key_released(move |_, _keyval, keycode, _state| {
                let event = InputEvent::Keyboard(KeyboardEvent::KeyUp {
                    scancode: keycode,
                    keycode: Some(keycode),
                    modifiers: 0, // TODO: Convert GTK modifiers
                });

                let adapter = adapter_key_release.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send key release: {}", e);
                    }
                });
            });
            obj.add_controller(key_controller);

            // Mouse wheel
            let scroll_controller =
                gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);

            let adapter_scroll = input_adapter.clone();
            scroll_controller.connect_scroll(move |_, dx, dy| {
                let event = InputEvent::Mouse(MouseEvent::Wheel {
                    delta_x: (dx * 120.0) as i32, // Convert to SPICE wheel units
                    delta_y: (dy * 120.0) as i32,
                });

                let adapter = adapter_scroll.clone();
                glib::spawn_future_local(async move {
                    if let Err(e) = adapter.send_event(event).await {
                        eprintln!("Failed to send wheel event: {}", e);
                    }
                });

                glib::Propagation::Stop
            });
            obj.add_controller(scroll_controller);
        }
    }
}
