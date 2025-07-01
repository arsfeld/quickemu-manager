use clap::Parser;
use spice_client::{
    SpiceClientShared,
    multimedia::{
        self, MultimediaBackend,
        display::{Display, DisplayMode},
        audio::AudioOutput,
        input::{InputHandler, InputEvent, KeyboardEvent, MouseEvent, MouseButton},
        spice_adapter::{SpiceDisplayAdapter, SpiceInputAdapter},
    },
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn, debug};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "rusty-spice")]
#[command(author, version, about = "A modern SPICE client written in Rust", long_about = None)]
struct Args {
    /// SPICE server host
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    /// SPICE server port
    #[arg(short, long, default_value = "5900")]
    port: u16,

    /// Enable fullscreen mode
    #[arg(short, long)]
    fullscreen: bool,

    /// Window width (default: 1024)
    #[arg(short = 'W', long, default_value_t = 1024)]
    width: u32,

    /// Window height (default: 768)  
    #[arg(short = 'h', long, default_value_t = 768)]
    height: u32,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Disable audio
    #[arg(long)]
    no_audio: bool,

    /// Disable input grab on focus
    #[arg(long)]
    no_grab: bool,

    /// Window title
    #[arg(short, long, default_value = "Rusty SPICE")]
    title: String,

    /// Password for SPICE connection
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Enable USB redirection
    #[arg(long)]
    enable_usbredir: bool,

    /// Enable clipboard sharing
    #[arg(long)]
    enable_clipboard: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialize logging
    let filter_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(filter_level)))
        .init();

    info!("Starting Rusty SPICE viewer...");
    info!("Connecting to {}:{}", args.host, args.port);

    // Create multimedia backend
    let backend = multimedia::create_default_backend()?;
    
    // Create display
    let mut display = backend.create_display()?;
    display.create_surface(DisplayMode {
        width: args.width,
        height: args.height,
        fullscreen: args.fullscreen,
    })?;
    display.set_title(&args.title)?;

    // Create audio if enabled
    let audio = if !args.no_audio {
        match backend.create_audio() {
            Ok(mut audio) => {
                audio.initialize(
                    multimedia::AudioSpec::default(),
                    multimedia::audio::AudioFormat::S16,
                )?;
                info!("Audio initialized");
                Some(audio)
            }
            Err(e) => {
                warn!("Failed to initialize audio: {}", e);
                None
            }
        }
    } else {
        info!("Audio disabled");
        None
    };

    // Create input handler
    let mut input = backend.create_input()?;
    if !args.no_grab {
        input.grab_input(true)?;
    }

    // Create SPICE client
    let mut spice_client = SpiceClientShared::new(args.host.clone(), args.port);
    
    if let Some(password) = args.password {
        spice_client.set_password(password).await;
    }

    if args.enable_usbredir {
        info!("USB redirection enabled");
        // TODO: Enable USB redirection when implemented
    }

    if args.enable_clipboard {
        info!("Clipboard sharing enabled");
        // TODO: Enable clipboard when implemented
    }

    // Connect to SPICE server
    info!("Connecting to SPICE server...");
    spice_client.connect().await?;
    info!("Connected successfully");

    // Start SPICE event loop
    spice_client.start_event_loop().await?;
    info!("SPICE event loop started");

    // Create SPICE display adapter
    let display_adapter = Arc::new(SpiceDisplayAdapter::new(
        spice_client.clone(),
        Box::new(display),
        0, // Primary display channel
    ));

    // Create SPICE input adapter
    let input_adapter = Arc::new(SpiceInputAdapter::new(
        spice_client.clone(),
        0, // Primary inputs channel
    ));

    // Main event loop
    let sdl_context = sdl2::init()
        .map_err(|e| format!("Failed to initialize SDL2: {}", e))?;
    let mut event_pump = sdl_context.event_pump()
        .map_err(|e| format!("Failed to create event pump: {}", e))?;
    
    let update_interval = std::time::Duration::from_millis(16); // ~60 FPS
    let mut last_update = std::time::Instant::now();
    
    'main_loop: loop {
        // Process SDL events
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;
            use sdl2::keyboard::Keycode;
            
            match event {
                Event::Quit { .. } => {
                    info!("Quit requested");
                    break 'main_loop;
                }
                Event::KeyDown { keycode: Some(Keycode::F11), .. } => {
                    let backend_display = display_adapter.get_backend_display();
                    let mut display = backend_display.lock().await;
                    display.toggle_fullscreen()?;
                }
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    let backend_display = display_adapter.get_backend_display();
                    let display = backend_display.lock().await;
                    if display.is_fullscreen() {
                        drop(display);
                        let mut display = backend_display.lock().await;
                        display.toggle_fullscreen()?;
                    }
                }
                Event::KeyDown { keycode, scancode, keymod, .. } => {
                    if let Some(scan) = scancode {
                        let event = InputEvent::Keyboard(KeyboardEvent::KeyDown {
                            scancode: scan as u32,
                            keycode: None, // SDL keycode not easily convertible
                            modifiers: 0, // TODO: Convert SDL modifiers
                        });
                        if let Err(e) = input_adapter.send_event(event).await {
                            warn!("Failed to send key down event: {}", e);
                        }
                    }
                }
                Event::KeyUp { keycode, scancode, keymod, .. } => {
                    if let Some(scan) = scancode {
                        let event = InputEvent::Keyboard(KeyboardEvent::KeyUp {
                            scancode: scan as u32,
                            keycode: None, // SDL keycode not easily convertible
                            modifiers: 0, // TODO: Convert SDL modifiers
                        });
                        if let Err(e) = input_adapter.send_event(event).await {
                            warn!("Failed to send key up event: {}", e);
                        }
                    }
                }
                Event::MouseMotion { x, y, xrel, yrel, .. } => {
                    let event = InputEvent::Mouse(MouseEvent::Motion {
                        x: x as u32,
                        y: y as u32,
                        relative_x: xrel,
                        relative_y: yrel,
                    });
                    if let Err(e) = input_adapter.send_event(event).await {
                        warn!("Failed to send mouse motion event: {}", e);
                    }
                }
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    let button = sdl_to_multimedia_button(mouse_btn);
                    let event = InputEvent::Mouse(MouseEvent::Button {
                        button,
                        pressed: true,
                        x: x as u32,
                        y: y as u32,
                    });
                    if let Err(e) = input_adapter.send_event(event).await {
                        warn!("Failed to send mouse button down event: {}", e);
                    }
                }
                Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                    let button = sdl_to_multimedia_button(mouse_btn);
                    let event = InputEvent::Mouse(MouseEvent::Button {
                        button,
                        pressed: false,
                        x: x as u32,
                        y: y as u32,
                    });
                    if let Err(e) = input_adapter.send_event(event).await {
                        warn!("Failed to send mouse button up event: {}", e);
                    }
                }
                Event::MouseWheel { x, y, .. } => {
                    let event = InputEvent::Mouse(MouseEvent::Wheel {
                        delta_x: x,
                        delta_y: y,
                    });
                    if let Err(e) = input_adapter.send_event(event).await {
                        warn!("Failed to send mouse wheel event: {}", e);
                    }
                }
                _ => {}
            }
        }

        // Update display with latest SPICE frame
        if last_update.elapsed() >= update_interval {
            if let Err(e) = display_adapter.update_display().await {
                warn!("Failed to update display: {}", e);
            }
            last_update = std::time::Instant::now();
        }

        // Small delay to prevent busy loop
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    info!("Shutting down...");
    
    // Disconnect from SPICE server
    spice_client.disconnect().await;
    info!("Disconnected from SPICE server");
    
    Ok(())
}

fn sdl_to_multimedia_button(button: sdl2::mouse::MouseButton) -> MouseButton {
    use sdl2::mouse::MouseButton as SdlButton;
    
    match button {
        SdlButton::Left => MouseButton::Left,
        SdlButton::Middle => MouseButton::Middle,
        SdlButton::Right => MouseButton::Right,
        SdlButton::X1 => MouseButton::X1,
        SdlButton::X2 => MouseButton::X2,
        _ => MouseButton::Left, // Default to left for unknown buttons
    }
}

