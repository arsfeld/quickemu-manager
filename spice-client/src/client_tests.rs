#[cfg(test)]
mod tests {
    use crate::client::SpiceClient;
    use crate::protocol::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_connection_timeout() {
        // Try to connect to a non-existent server with a short timeout
        let mut client = SpiceClient::new("127.0.0.1".to_string(), 9999);

        let connect_future = client.connect();
        let result = timeout(Duration::from_secs(2), connect_future).await;

        // Should timeout or get connection refused
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_reconnection_after_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = connected.clone();

        // Start a server that accepts one connection then closes
        let server_task = tokio::spawn(async move {
            // First connection
            let (mut socket, _) = listener.accept().await.unwrap();
            connected_clone.store(true, Ordering::SeqCst);

            // Complete handshake
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();

            let mut link_msg_buf = [0u8; 18];
            socket.read_exact(&mut link_msg_buf).await.unwrap();

            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };

            let reply_bytes = bincode::serialize(&reply).unwrap();
            socket.write_all(&reply_bytes).await.unwrap();

            // Send init message
            let init_header = SpiceDataHeader {
                serial: 1,
                msg_type: SPICE_MSG_MAIN_INIT,
                msg_size: 32,
                sub_list: 0,
            };

            let header_bytes = bincode::serialize(&init_header).unwrap();
            socket.write_all(&header_bytes).await.unwrap();

            let init_msg = SpiceMsgMainInit {
                session_id: 1,
                display_channels_hint: 1,
                supported_mouse_modes: 3,
                current_mouse_mode: 2,
                agent_connected: 0,
                agent_tokens: 0,
                multi_media_time: 0,
                ram_hint: 0,
            };

            let init_bytes = bincode::serialize(&init_msg).unwrap();
            socket.write_all(&init_bytes).await.unwrap();

            // Close connection after a short delay
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(socket);

            // Accept second connection attempt
            let (mut socket, _) = listener.accept().await.unwrap();

            // Complete handshake again
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();

            let mut link_msg_buf = [0u8; 18];
            socket.read_exact(&mut link_msg_buf).await.unwrap();

            socket.write_all(&reply_bytes).await.unwrap();
            socket.write_all(&header_bytes).await.unwrap();
            socket.write_all(&init_bytes).await.unwrap();

            // Keep connection open longer this time
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        let mut client = SpiceClient::new("127.0.0.1".to_string(), port);

        // First connection should succeed
        client.connect().await.unwrap();
        assert!(connected.load(Ordering::SeqCst));

        // Wait for server to close connection
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Disconnect and reconnect
        client.disconnect();

        // Second connection should also succeed
        client.connect().await.unwrap();

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_display_channels() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        let server_task = tokio::spawn(async move {
            // Accept main channel connection
            let (mut main_socket, _) = listener.accept().await.unwrap();

            // Complete main channel handshake
            let mut header_buf = [0u8; 16];
            main_socket.read_exact(&mut header_buf).await.unwrap();

            let mut link_msg_buf = [0u8; 18];
            main_socket.read_exact(&mut link_msg_buf).await.unwrap();

            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };

            let reply_bytes = bincode::serialize(&reply).unwrap();
            main_socket.write_all(&reply_bytes).await.unwrap();

            // Send init message
            let init_header = SpiceDataHeader {
                serial: 1,
                msg_type: SPICE_MSG_MAIN_INIT,
                msg_size: 32,
                sub_list: 0,
            };

            let header_bytes = bincode::serialize(&init_header).unwrap();
            main_socket.write_all(&header_bytes).await.unwrap();

            let init_msg = SpiceMsgMainInit {
                session_id: 1,
                display_channels_hint: 2, // Hint for 2 display channels
                supported_mouse_modes: 3,
                current_mouse_mode: 2,
                agent_connected: 0,
                agent_tokens: 0,
                multi_media_time: 0,
                ram_hint: 0,
            };

            let init_bytes = bincode::serialize(&init_msg).unwrap();
            main_socket.write_all(&init_bytes).await.unwrap();

            // Wait for channels list request
            let mut msg_header_buf = [0u8; 24];
            main_socket.read_exact(&mut msg_header_buf).await.unwrap();

            // Send channels list with multiple display channels
            let channels_header = SpiceDataHeader {
                serial: 2,
                msg_type: SPICE_MSG_MAIN_CHANNELS_LIST,
                msg_size: 12, // 3 channels * 4 bytes each
                sub_list: 0,
            };

            let header_bytes = bincode::serialize(&channels_header).unwrap();
            main_socket.write_all(&header_bytes).await.unwrap();

            // Send channel list data: Display 0, Display 1, Inputs 0
            let channels_data = vec![
                0x02, 0x00, 0x00, 0x00, // Display channel type = 2, id = 0
                0x02, 0x01, 0x00, 0x00, // Display channel type = 2, id = 1
                0x03, 0x00, 0x00, 0x00, // Inputs channel type = 3, id = 0
            ];
            main_socket.write_all(&channels_data).await.unwrap();

            // Accept display channel connections
            for _ in 0..2 {
                let (mut display_socket, _) = listener.accept().await.unwrap();

                // Complete display channel handshake
                let mut header_buf = [0u8; 16];
                display_socket.read_exact(&mut header_buf).await.unwrap();

                let mut link_msg_buf = [0u8; 18];
                display_socket.read_exact(&mut link_msg_buf).await.unwrap();

                display_socket.write_all(&reply_bytes).await.unwrap();

                // Send display mode
                let mode_header = SpiceDataHeader {
                    serial: 1,
                    msg_type: SPICE_MSG_DISPLAY_MODE,
                    msg_size: 20,
                    sub_list: 0,
                };

                let header_bytes = bincode::serialize(&mode_header).unwrap();
                display_socket.write_all(&header_bytes).await.unwrap();

                let mode_data = vec![
                    0x00, 0x04, 0x00, 0x00, // x = 1024
                    0x00, 0x03, 0x00, 0x00, // y = 768
                    0x20, 0x00, 0x00, 0x00, // bits = 32
                    0x00, 0x00, 0x00, 0x00, // reserved
                    0x00, 0x00, 0x00, 0x00, // reserved2
                ];
                display_socket.write_all(&mode_data).await.unwrap();
            }

            // Keep connections alive
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        let mut client = SpiceClient::new("127.0.0.1".to_string(), port);
        client.connect().await.unwrap();

        // Give time for channels to be established
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check that we have surfaces for both display channels
        let surface0 = client.get_display_surface(0).await;
        let surface1 = client.get_display_surface(1).await;

        assert!(surface0.is_some());
        assert!(surface1.is_some());

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_event_loop_error_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();

            // Complete handshake
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();

            let mut link_msg_buf = [0u8; 18];
            socket.read_exact(&mut link_msg_buf).await.unwrap();

            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };

            let reply_bytes = bincode::serialize(&reply).unwrap();
            socket.write_all(&reply_bytes).await.unwrap();

            // Send init message
            let init_header = SpiceDataHeader {
                serial: 1,
                msg_type: SPICE_MSG_MAIN_INIT,
                msg_size: 32,
                sub_list: 0,
            };

            let header_bytes = bincode::serialize(&init_header).unwrap();
            socket.write_all(&header_bytes).await.unwrap();

            let init_msg = SpiceMsgMainInit {
                session_id: 1,
                display_channels_hint: 0,
                supported_mouse_modes: 3,
                current_mouse_mode: 2,
                agent_connected: 0,
                agent_tokens: 0,
                multi_media_time: 0,
                ram_hint: 0,
            };

            let init_bytes = bincode::serialize(&init_msg).unwrap();
            socket.write_all(&init_bytes).await.unwrap();

            // Send invalid message to trigger error
            tokio::time::sleep(Duration::from_millis(100)).await;

            let invalid_header = SpiceDataHeader {
                serial: 2,
                msg_type: 9999, // Invalid message type
                msg_size: 4,
                sub_list: 0,
            };

            let header_bytes = bincode::serialize(&invalid_header).unwrap();
            socket.write_all(&header_bytes).await.unwrap();
            socket.write_all(&[0, 0, 0, 0]).await.unwrap();

            // Keep connection open
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        let mut client = SpiceClient::new("127.0.0.1".to_string(), port);
        client.connect().await.unwrap();

        // Start event loop
        let event_loop_result = client.start_event_loop().await;

        // Event loop should handle invalid messages gracefully
        assert!(event_loop_result.is_ok());

        server_task.await.unwrap();
    }
}
