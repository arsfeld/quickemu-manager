use clap::Parser;
use gtk4::prelude::*;
use gtk4::{gio, glib, Application, ApplicationWindow, DrawingArea, HeaderBar, Orientation};

// Type alias to avoid conflict with std::boxed::Box
type GtkBox = gtk4::Box;
use spice_client::{
    multimedia::{
        self,
        display::Display,
        input::{InputEvent, KeyboardEvent, MouseButton, MouseEvent},
        spice_adapter::{SpiceDisplayAdapter, SpiceInputAdapter},
        MultimediaBackend,
    },
    SpiceClientShared,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug, Clone)]
#[command(name = "rusty-spice-gtk")]
#[command(author, version, about = "GTK4-based SPICE client", long_about = None)]
struct Args {
    /// SPICE server host
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    /// SPICE server port
    #[arg(short, long, default_value = "5900")]
    port: u16,

    /// Window width (default: 1024)
    #[arg(short = 'w', long, default_value_t = 1024)]
    width: u32,

    /// Window height (default: 768)  
    #[arg(long, default_value_t = 768)]
    height: u32,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Password for SPICE connection
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Window title
    #[arg(short, long, default_value = "Rusty SPICE - GTK4")]
    title: String,
}

struct SpiceWindow {
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    spice_client: SpiceClientShared,
    display_adapter: Arc<Mutex<Option<SpiceDisplayAdapter>>>,
    input_adapter: Arc<Mutex<Option<SpiceInputAdapter>>>,
    gtk_display: Arc<Mutex<Option<Box<dyn Display + Send>>>>,
}

impl SpiceWindow {
    fn new(
        window: ApplicationWindow,
        spice_client: SpiceClientShared,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create GTK4 backend
        let backend = multimedia::gtk4::Gtk4Backend::new()?;
        let mut gtk_display = backend.create_display()?;

        // Initialize display with default mode
        gtk_display.create_surface(multimedia::display::DisplayMode {
            width: 1024,
            height: 768,
            fullscreen: false,
        })?;

        // Get the drawing area from the GTK4 display
        let drawing_area = if let Some(display) = gtk_display
            .as_any()
            .downcast_ref::<multimedia::gtk4::display::Gtk4Display>(
        ) {
            display
                .get_drawing_area()
                .ok_or("Failed to get drawing area from GTK4 display")?
                .clone()
        } else {
            return Err("Failed to downcast to Gtk4Display".into());
        };

        // Set window reference in the display
        if let Some(display) = gtk_display
            .as_any_mut()
            .downcast_mut::<multimedia::gtk4::display::Gtk4Display>()
        {
            display.set_window(window.clone().upcast());
        }

        drawing_area.set_can_focus(true);
        drawing_area.set_focusable(true);

        Ok(Self {
            window,
            drawing_area,
            spice_client,
            display_adapter: Arc::new(Mutex::new(None)),
            input_adapter: Arc::new(Mutex::new(None)),
            gtk_display: Arc::new(Mutex::new(Some(Box::new(gtk_display)))),
        })
    }

    fn setup_event_handlers(&self) {
        // Keyboard event controller
        let key_controller = gtk4::EventControllerKey::new();
        let input_adapter = self.input_adapter.clone();

        key_controller.connect_key_pressed(move |_, keyval, keycode, state| {
            debug!(
                "Key pressed: keyval={}, keycode={}, state={:?}",
                keyval, keycode, state
            );

            let event = InputEvent::Keyboard(KeyboardEvent::KeyDown {
                scancode: keycode,
                keycode: None, // GTK Key type can't be converted to u32
                modifiers: state.bits(),
            });

            // Forward to SPICE via input adapter
            let input_adapter = input_adapter.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send key down event: {}", e);
                    }
                }
            });

            glib::Propagation::Proceed
        });

        let input_adapter_clone = self.input_adapter.clone();
        key_controller.connect_key_released(move |_, keyval, keycode, state| {
            debug!(
                "Key released: keyval={}, keycode={}, state={:?}",
                keyval, keycode, state
            );

            let event = InputEvent::Keyboard(KeyboardEvent::KeyUp {
                scancode: keycode,
                keycode: None, // GTK Key type can't be converted to u32
                modifiers: state.bits(),
            });

            // Forward to SPICE via input adapter
            let input_adapter_clone = input_adapter_clone.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter_clone.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send key up event: {}", e);
                    }
                }
            });
        });

        self.drawing_area.add_controller(key_controller);

        // Mouse motion controller
        let motion_controller = gtk4::EventControllerMotion::new();
        let input_adapter = self.input_adapter.clone();

        motion_controller.connect_motion(move |_, x, y| {
            let event = InputEvent::Mouse(MouseEvent::Motion {
                x: x as u32,
                y: y as u32,
                relative_x: 0, // GTK doesn't provide relative motion easily
                relative_y: 0,
            });

            let input_adapter = input_adapter.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send mouse motion event: {}", e);
                    }
                }
            });
        });

        self.drawing_area.add_controller(motion_controller);

        // Mouse button controller
        let click_controller = gtk4::GestureClick::new();
        let input_adapter = self.input_adapter.clone();

        click_controller.connect_pressed(move |gesture, n_press, x, y| {
            let button = convert_gtk_button(gesture.current_button());
            debug!("Mouse button {:?} pressed at ({}, {})", button, x, y);

            let event = InputEvent::Mouse(MouseEvent::Button {
                button,
                pressed: true,
                x: x as u32,
                y: y as u32,
            });

            let input_adapter = input_adapter.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send mouse button down event: {}", e);
                    }
                }
            });
        });

        let input_adapter_clone = self.input_adapter.clone();
        click_controller.connect_released(move |gesture, n_press, x, y| {
            let button = convert_gtk_button(gesture.current_button());
            debug!("Mouse button {:?} released at ({}, {})", button, x, y);

            let event = InputEvent::Mouse(MouseEvent::Button {
                button,
                pressed: false,
                x: x as u32,
                y: y as u32,
            });

            let input_adapter_clone = input_adapter_clone.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter_clone.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send mouse button up event: {}", e);
                    }
                }
            });
        });

        self.drawing_area.add_controller(click_controller);

        // Scroll controller
        let scroll_controller =
            gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        let input_adapter = self.input_adapter.clone();

        scroll_controller.connect_scroll(move |_, dx, dy| {
            debug!("Scroll: dx={}, dy={}", dx, dy);

            let event = InputEvent::Mouse(MouseEvent::Wheel {
                delta_x: dx as i32,
                delta_y: dy as i32,
            });

            let input_adapter = input_adapter.clone();
            glib::spawn_future_local(async move {
                let adapter = input_adapter.lock().await;
                if let Some(ref adapter) = *adapter {
                    if let Err(e) = adapter.send_event(event).await {
                        warn!("Failed to send scroll event: {}", e);
                    }
                }
            });

            glib::Propagation::Proceed
        });

        self.drawing_area.add_controller(scroll_controller);
    }
}

fn build_ui(app: &Application, args: Args) {
    // Create main window
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&args.title)
        .default_width(args.width as i32)
        .default_height(args.height as i32)
        .build();

    // Create header bar
    let header_bar = HeaderBar::builder()
        .title_widget(&gtk4::Label::new(Some(&args.title)))
        .show_title_buttons(true)
        .build();

    window.set_titlebar(Some(&header_bar));

    // Create SPICE client
    let spice_client = SpiceClientShared::new(args.host.clone(), args.port);

    // Create SpiceWindow
    let spice_window = match SpiceWindow::new(window.clone(), spice_client) {
        Ok(sw) => Rc::new(RefCell::new(sw)),
        Err(e) => {
            error!("Failed to create SpiceWindow: {}", e);
            let dialog = gtk4::MessageDialog::new(
                Some(&window),
                gtk4::DialogFlags::DESTROY_WITH_PARENT,
                gtk4::MessageType::Error,
                gtk4::ButtonsType::Close,
                &format!("Failed to initialize: {}", e),
            );
            dialog.connect_response(|dialog, _| dialog.close());
            dialog.present();
            return;
        }
    };

    // Setup main container
    let main_box = GtkBox::new(Orientation::Vertical, 0);

    // Add drawing area
    let drawing_area = spice_window.borrow().drawing_area.clone();
    drawing_area.set_vexpand(true);
    drawing_area.set_hexpand(true);
    main_box.append(&drawing_area);

    // Setup event handlers
    spice_window.borrow().setup_event_handlers();

    // Add status bar
    let status_bar = gtk4::Label::builder()
        .label(&format!("Connecting to {}:{}...", args.host, args.port))
        .xalign(0.0)
        .margin_start(6)
        .margin_end(6)
        .margin_top(3)
        .margin_bottom(3)
        .build();
    main_box.append(&status_bar);

    window.set_child(Some(&main_box));

    // Connect to SPICE server asynchronously
    let password = args.password.clone();
    let spice_window_clone = spice_window.clone();
    let status_bar_clone = status_bar.clone();
    let display_adapter_ref = spice_window.borrow().display_adapter.clone();

    // Use glib's async runtime instead of spawning a new thread
    glib::spawn_future_local(async move {
        // Use the existing SPICE client
        let mut client = spice_window_clone.borrow().spice_client.clone();

        // Set password if provided
        if let Some(pwd) = password {
            client.set_password(pwd).await;
        }

        // Connect in a background thread with Tokio runtime
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Result<(), String>>();
        let client_for_thread = client.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                info!("Connecting to SPICE server...");
                match client_for_thread.connect().await {
                    Ok(()) => {
                        info!("Connected successfully");
                        // Start event loop
                        if let Err(e) = client_for_thread.start_event_loop().await {
                            error!("Event loop error: {}", e);
                            let _ = tx.send(Err(format!("Event loop error: {}", e)));
                        } else {
                            let _ = tx.send(Ok(()));
                        }
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                        let _ = tx.send(Err(format!("Connection failed: {}", e)));
                    }
                }
            });
        });

        // Handle connection result
        let client_for_adapters = client.clone(); // Keep a clone for the adapters
        glib::spawn_future_local(async move {
            if let Some(result) = rx.recv().await {
                match result {
                    Ok(()) => {
                        status_bar_clone.set_label("Connected to SPICE server");

                        // Client is already set up, no need to update

                        // Set up adapters
                        glib::spawn_future_local({
                            let spice_window = spice_window_clone.clone();
                            let display_adapter_ref = display_adapter_ref.clone();
                            let client = client_for_adapters.clone(); // Use the cloned client
                            async move {
                                // Create display adapter
                                let gtk_display_arc = spice_window.borrow().gtk_display.clone();
                                let mut display_guard = gtk_display_arc.lock().await;
                                if let Some(display) = display_guard.take() {
                                    let display_adapter = SpiceDisplayAdapter::new(
                                        client.clone(),
                                        display,
                                        0, // Primary display channel
                                    );

                                    let display_adapter_arc =
                                        spice_window.borrow().display_adapter.clone();
                                    let mut adapter_guard = display_adapter_arc.lock().await;
                                    *adapter_guard = Some(display_adapter);
                                }

                                // Create input adapter
                                let input_adapter = SpiceInputAdapter::new(
                                    client, 0, // Primary inputs channel
                                );

                                let input_adapter_arc = spice_window.borrow().input_adapter.clone();
                                let mut adapter_guard = input_adapter_arc.lock().await;
                                *adapter_guard = Some(input_adapter);

                                // Start display update timer
                                glib::timeout_add_local(
                                    std::time::Duration::from_millis(16),
                                    move || {
                                        let adapter = display_adapter_ref.clone();
                                        glib::spawn_future_local(async move {
                                            let adapter_guard = adapter.lock().await;
                                            if let Some(ref adapter) = *adapter_guard {
                                                if let Err(e) = adapter.update_display().await {
                                                    debug!("Failed to update display: {}", e);
                                                }
                                            }
                                        });
                                        glib::ControlFlow::Continue
                                    },
                                );
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to connect to SPICE server: {}", e);
                        status_bar_clone.set_label(&format!("Connection failed: {}", e));
                    }
                }
            }
        });
    });

    // Setup keyboard shortcuts
    let action_fullscreen = gio::SimpleAction::new("fullscreen", None);
    let window_clone = window.clone();
    action_fullscreen.connect_activate(move |_, _| {
        if window_clone.is_fullscreen() {
            window_clone.unfullscreen();
        } else {
            window_clone.fullscreen();
        }
    });
    app.add_action(&action_fullscreen);
    app.set_accels_for_action("app.fullscreen", &["F11"]);

    window.present();
}

fn convert_gtk_button(button: u32) -> MouseButton {
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        8 => MouseButton::X1,
        9 => MouseButton::X2,
        _ => MouseButton::Left,
    }
}

fn main() -> glib::ExitCode {
    // Parse arguments before GTK initialization
    let args = Args::parse();

    // Initialize logging
    let filter_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter_level)))
        .init();

    info!("Starting Rusty SPICE GTK4 client");

    // Create GTK4 application with flags to disable argument handling
    let app = Application::builder()
        .application_id("com.rustysoftware.spice-client-gtk")
        .flags(gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(move |app| {
        build_ui(app, args.clone());
    });

    // Run with empty args to prevent GTK from parsing command line
    let empty_args: Vec<String> = vec![];
    app.run_with_args(&empty_args)
}
