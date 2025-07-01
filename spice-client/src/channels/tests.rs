#[cfg(test)]
mod tests {
    use crate::channels::ChannelConnection;
    use crate::error::SpiceError;
    use crate::protocol::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_channel_handshake_invalid_magic() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            
            // Read link header
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();
            
            // Read link message (size is in header bytes 12-15)
            let size = u32::from_le_bytes([header_buf[12], header_buf[13], header_buf[14], header_buf[15]]);
            let mut link_msg_buf = vec![0u8; size as usize];
            socket.read_exact(&mut link_msg_buf).await.unwrap();
            
            // Send invalid magic in reply
            let invalid_reply = SpiceLinkReply {
                magic: 0xDEADBEEF, // Invalid magic
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };
            
            use binrw::BinWrite;
            let mut reply_cursor = std::io::Cursor::new(Vec::new());
            invalid_reply.write(&mut reply_cursor).unwrap();
            let reply_bytes = reply_cursor.into_inner();
            socket.write_all(&reply_bytes).await.unwrap();
        });

        // Try to connect
        let result = ChannelConnection::new(
            &addr.ip().to_string(),
            addr.port(),
            ChannelType::Main,
            0,
        ).await;

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                SpiceError::Protocol(msg) => {
                    assert!(msg.contains("Invalid") || msg.contains("magic"));
                }
                _ => panic!("Expected Protocol error, got {:?}", e),
            }
        }

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_channel_handshake_version_mismatch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            
            // Read link header
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();
            
            // Read link message (size is in header bytes 12-15)
            let size = u32::from_le_bytes([header_buf[12], header_buf[13], header_buf[14], header_buf[15]]);
            let mut link_msg_buf = vec![0u8; size as usize];
            socket.read_exact(&mut link_msg_buf).await.unwrap();
            
            // Send version mismatch reply
            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: 1, // Old version
                minor_version: 0,
                size: 0,
            };
            
            use binrw::BinWrite;
            let mut reply_cursor = std::io::Cursor::new(Vec::new());
            reply.write(&mut reply_cursor).unwrap();
            let reply_bytes = reply_cursor.into_inner();
            socket.write_all(&reply_bytes).await.unwrap();
        });

        // Try to connect
        let result = ChannelConnection::new(
            &addr.ip().to_string(),
            addr.port(),
            ChannelType::Main,
            0,
        ).await;

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                SpiceError::VersionMismatch { expected, actual } => {
                    assert_eq!(expected, SPICE_VERSION_MAJOR);
                    assert_eq!(actual, 1);
                }
                _ => panic!("Expected VersionMismatch error, got {:?}", e),
            }
        }

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_channel_handshake_connection_refused() {
        // Try to connect to a port that's not listening
        let result = ChannelConnection::new(
            "127.0.0.1",
            9999, // Unlikely to be in use
            ChannelType::Display,
            0,
        ).await;

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                SpiceError::Io(_) => {
                    // Expected IO error
                }
                _ => panic!("Expected Io error, got {:?}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_channel_handshake_partial_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            
            // Read link header
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();
            
            // Read link message (size is in header bytes 12-15)
            let size = u32::from_le_bytes([header_buf[12], header_buf[13], header_buf[14], header_buf[15]]);
            let mut link_msg_buf = vec![0u8; size as usize];
            socket.read_exact(&mut link_msg_buf).await.unwrap();
            
            // Send only partial reply (8 bytes instead of 16)
            let partial_data = [0x43, 0x49, 0x50, 0x53, 0x02, 0x00, 0x00, 0x00];
            socket.write_all(&partial_data).await.unwrap();
            
            // Close connection
            drop(socket);
        });

        // Try to connect
        let result = ChannelConnection::new(
            &addr.ip().to_string(),
            addr.port(),
            ChannelType::Main,
            0,
        ).await;

        assert!(result.is_err());
        
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_send_message_disconnected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            
            // Complete handshake
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();
            
            let mut link_msg_buf = [0u8; 20];  // SpiceLinkMess is 20 bytes
            socket.read_exact(&mut link_msg_buf).await.unwrap();
            
            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };
            
            use binrw::BinWrite;
            let mut reply_cursor = std::io::Cursor::new(Vec::new());
            reply.write(&mut reply_cursor).unwrap();
            let reply_bytes = reply_cursor.into_inner();
            socket.write_all(&reply_bytes).await.unwrap();
            
            // Immediately close connection
            drop(socket);
        });

        // Connect successfully
        let mut channel = ChannelConnection::new(
            &addr.ip().to_string(),
            addr.port(),
            ChannelType::Main,
            0,
        ).await.unwrap();

        // Wait for server to close
        server_task.await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Try to send message on closed connection
        let result = channel.send_message(100, &[1, 2, 3, 4]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_message_invalid_header() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            
            // Complete handshake
            let mut header_buf = [0u8; 16];
            socket.read_exact(&mut header_buf).await.unwrap();
            
            let mut link_msg_buf = [0u8; 20];  // SpiceLinkMess is 20 bytes
            socket.read_exact(&mut link_msg_buf).await.unwrap();
            
            let reply = SpiceLinkReply {
                magic: SPICE_MAGIC,
                major_version: SPICE_VERSION_MAJOR,
                minor_version: SPICE_VERSION_MINOR,
                size: 0,
            };
            
            use binrw::BinWrite;
            let mut reply_cursor = std::io::Cursor::new(Vec::new());
            reply.write(&mut reply_cursor).unwrap();
            let reply_bytes = reply_cursor.into_inner();
            socket.write_all(&reply_bytes).await.unwrap();
            
            // Send invalid message header (too short)
            let invalid_header = [0u8; 10];
            socket.write_all(&invalid_header).await.unwrap();
        });

        // Connect successfully
        let mut channel = ChannelConnection::new(
            &addr.ip().to_string(),
            addr.port(),
            ChannelType::Main,
            0,
        ).await.unwrap();

        // Try to read message with invalid header
        let result = channel.read_message().await;
        assert!(result.is_err());
        
        server_task.await.unwrap();
    }
}