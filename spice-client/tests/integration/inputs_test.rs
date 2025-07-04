use spice_client::channels::{InputEvent, InputsChannel, KeyCode, MouseButton, MouseMode};
use spice_client::test_utils::MockSpiceServer;
use tokio::time::Duration;

#[tokio::test]
async fn test_inputs_channel_connection() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    channel.initialize().await.unwrap();

    // Verify initial state
    assert_eq!(channel.get_mouse_mode(), MouseMode::Server);
    let modifiers = channel.get_modifiers();
    assert!(!modifiers.shift);
    assert!(!modifiers.ctrl);
    assert!(!modifiers.alt);
}

#[tokio::test]
async fn test_keyboard_events() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Test key press events
    let keys = vec![
        KeyCode::Char('A'),
        KeyCode::Char('1'),
        KeyCode::Enter,
        KeyCode::Escape,
        KeyCode::Space,
    ];

    for key in keys {
        // Key down
        let result = channel.send_event(InputEvent::KeyDown(key)).await;
        assert!(result.is_ok(), "Key down event should succeed");

        // Key up
        let result = channel.send_event(InputEvent::KeyUp(key)).await;
        assert!(result.is_ok(), "Key up event should succeed");
    }
}

#[tokio::test]
async fn test_mouse_movement() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Test mouse movement events
    let positions = vec![(100, 100), (200, 200), (300, 300), (0, 0), (1920, 1080)];

    for (x, y) in positions {
        let result = channel.send_event(InputEvent::MouseMove { x, y }).await;
        assert!(result.is_ok(), "Mouse move event should succeed");
    }
}

#[tokio::test]
async fn test_mouse_buttons() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Test all mouse buttons
    let buttons = vec![MouseButton::Left, MouseButton::Middle, MouseButton::Right];

    for button in buttons {
        // Press
        let result = channel
            .send_event(InputEvent::MouseButton {
                button,
                pressed: true,
            })
            .await;
        assert!(result.is_ok(), "Mouse button press should succeed");

        // Release
        let result = channel
            .send_event(InputEvent::MouseButton {
                button,
                pressed: false,
            })
            .await;
        assert!(result.is_ok(), "Mouse button release should succeed");
    }
}

#[tokio::test]
async fn test_modifier_keys() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Test modifier keys
    let modifiers = vec![
        KeyCode::Other(0x2A), // Left Shift
        KeyCode::Other(0x1D), // Left Ctrl
        KeyCode::Other(0x38), // Left Alt
    ];

    for modifier in modifiers {
        // Press modifier
        let result = channel.send_event(InputEvent::KeyDown(modifier)).await;
        assert!(result.is_ok());

        // Type a key with modifier held
        let result = channel
            .send_event(InputEvent::KeyDown(KeyCode::Char('A')))
            .await;
        assert!(result.is_ok());

        let result = channel
            .send_event(InputEvent::KeyUp(KeyCode::Char('A')))
            .await;
        assert!(result.is_ok());

        // Release modifier
        let result = channel.send_event(InputEvent::KeyUp(modifier)).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_complex_input_sequence() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = InputsChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Simulate typing "Hello World!" with mouse clicks
    let sequence = vec![
        // Move mouse to text field
        InputEvent::MouseMove { x: 500, y: 300 },
        // Click to focus
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        },
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: false,
        },
        // Type "Hello"
        InputEvent::KeyDown(KeyCode::Char('H')),
        InputEvent::KeyUp(KeyCode::Char('H')),
        InputEvent::KeyDown(KeyCode::Char('e')),
        InputEvent::KeyUp(KeyCode::Char('e')),
        InputEvent::KeyDown(KeyCode::Char('l')),
        InputEvent::KeyUp(KeyCode::Char('l')),
        InputEvent::KeyDown(KeyCode::Char('l')),
        InputEvent::KeyUp(KeyCode::Char('l')),
        InputEvent::KeyDown(KeyCode::Char('o')),
        InputEvent::KeyUp(KeyCode::Char('o')),
        // Space
        InputEvent::KeyDown(KeyCode::Space),
        InputEvent::KeyUp(KeyCode::Space),
        // Type "World"
        InputEvent::KeyDown(KeyCode::Char('W')),
        InputEvent::KeyUp(KeyCode::Char('W')),
        InputEvent::KeyDown(KeyCode::Char('o')),
        InputEvent::KeyUp(KeyCode::Char('o')),
        InputEvent::KeyDown(KeyCode::Char('r')),
        InputEvent::KeyUp(KeyCode::Char('r')),
        InputEvent::KeyDown(KeyCode::Char('l')),
        InputEvent::KeyUp(KeyCode::Char('l')),
        InputEvent::KeyDown(KeyCode::Char('d')),
        InputEvent::KeyUp(KeyCode::Char('d')),
        // Shift+1 for "!"
        InputEvent::KeyDown(KeyCode::Other(0x2A)), // Shift
        InputEvent::KeyDown(KeyCode::Char('1')),
        InputEvent::KeyUp(KeyCode::Char('1')),
        InputEvent::KeyUp(KeyCode::Other(0x2A)), // Shift
        // Enter
        InputEvent::KeyDown(KeyCode::Enter),
        InputEvent::KeyUp(KeyCode::Enter),
    ];

    for event in sequence {
        let result = channel.send_event(event).await;
        assert!(result.is_ok(), "Event {:?} should succeed", event);

        // Small delay between events
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_inputs_channel() {
    // Test WebAssembly-specific inputs channel creation
    let mut channel = InputsChannel::new_websocket("ws://localhost:5900", 0)
        .await
        .unwrap_or_else(|_| panic!("Failed to create WASM inputs channel"));

    // Test sending events
    let result = channel
        .send_event(InputEvent::KeyDown(KeyCode::Char('A')))
        .await;
    assert!(result.is_ok());

    let result = channel
        .send_event(InputEvent::MouseMove { x: 100, y: 100 })
        .await;
    assert!(result.is_ok());
}
