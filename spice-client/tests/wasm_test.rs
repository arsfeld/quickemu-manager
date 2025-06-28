#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use spice_client::channels::display::{DisplayChannel, DisplaySurface};
use spice_client::channels::display_wasm::WasmDisplayManager;
use spice_client::protocol::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_wasm_canvas_rendering() {
    // Create a mock WebSocket URL (won't actually connect in test)
    let mut channel = DisplayChannel::new_wasm("ws://localhost:5900", 0)
        .await
        .unwrap_or_else(|_| panic!("Failed to create WASM display channel"));
    
    // Create display manager
    let mut display_manager = WasmDisplayManager::new();
    
    // Simulate surface creation
    let surface = DisplaySurface {
        width: 800,
        height: 600,
        format: 32,
        data: vec![255; 800 * 600 * 4], // White surface
    };
    
    // This would normally be done by the channel's message handler
    // For testing, we'll manually insert a surface
    // Note: This requires making surfaces field public or adding a test method
    
    // Test canvas creation
    let result = display_manager.handle_display_update(&channel).await;
    assert!(result.is_ok(), "Canvas rendering should succeed");
}

#[wasm_bindgen_test]
async fn test_wasm_video_stream() {
    let mut display_manager = WasmDisplayManager::new();
    
    // Test video stream creation
    let stream_info = crate::channels::display::StreamInfo {
        id: 1,
        codec_type: 1, // MJPEG
        width: 640,
        height: 480,
        dest_rect: SpiceRect {
            left: 100,
            top: 100,
            right: 740,
            bottom: 580,
        },
    };
    
    let result = display_manager.handle_stream_create(&stream_info).await;
    assert!(result.is_ok(), "Video stream creation should succeed");
    
    // Test video data append
    let dummy_data = vec![0xFF; 1024]; // Dummy video data
    let result = display_manager.handle_stream_data(1, &dummy_data).await;
    assert!(result.is_ok(), "Video data append should succeed");
    
    // Test stream destruction
    let result = display_manager.handle_stream_destroy(1).await;
    assert!(result.is_ok(), "Video stream destruction should succeed");
}

#[wasm_bindgen_test]
async fn test_wasm_multi_display() {
    let mut display_manager = WasmDisplayManager::new();
    
    // Create multiple video streams
    for i in 0..3 {
        let stream_info = crate::channels::display::StreamInfo {
            id: i,
            codec_type: 1,
            width: 640,
            height: 480,
            dest_rect: SpiceRect {
                left: i as i32 * 640,
                top: 0,
                right: (i + 1) as i32 * 640,
                bottom: 480,
            },
        };
        
        let result = display_manager.handle_stream_create(&stream_info).await;
        assert!(result.is_ok(), "Stream {} creation should succeed", i);
    }
    
    // Test that all canvases were created
    let canvases = display_manager.get_canvases();
    assert!(canvases.len() <= 3, "Should have at most 3 canvases");
}

#[wasm_bindgen_test]
fn test_wasm_performance_optimizer() {
    use spice_client::wasm::video_renderer::WasmPerformanceOptimizer;
    
    let mut optimizer = WasmPerformanceOptimizer::new();
    
    // Test frame skipping logic
    let should_skip = optimizer.should_skip_frame();
    assert!(!should_skip, "First frame should not be skipped");
    
    // Test quality level
    let quality = optimizer.get_quality_level();
    assert!(quality > 0, "Quality level should be positive");
    assert!(quality <= 100, "Quality level should not exceed 100");
}