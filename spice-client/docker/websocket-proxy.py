#!/usr/bin/env python3
import asyncio
import websockets
import socket
import logging
import os

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

WS_PORT = int(os.environ.get('WS_PORT', '5959'))
SPICE_HOST = os.environ.get('SPICE_HOST', 'qemu-spice')
SPICE_PORT = int(os.environ.get('SPICE_PORT', '5900'))

async def tcp_to_ws(tcp_socket, websocket):
    """Forward data from TCP to WebSocket"""
    loop = asyncio.get_event_loop()
    try:
        while True:
            # Read from TCP socket in executor to avoid blocking
            data = await loop.run_in_executor(None, tcp_socket.recv, 65536)
            if not data:
                logger.info("TCP connection closed")
                break
            await websocket.send(data)
            logger.debug(f"TCP -> WS: {len(data)} bytes")
    except Exception as e:
        logger.error(f"TCP to WS error: {e}")

async def ws_to_tcp(websocket, tcp_socket):
    """Forward data from WebSocket to TCP"""
    try:
        while True:
            data = await websocket.recv()
            if isinstance(data, str):
                data = data.encode()
            tcp_socket.sendall(data)
            logger.debug(f"WS -> TCP: {len(data)} bytes")
    except websockets.exceptions.ConnectionClosed:
        logger.info("WebSocket closed")
    except Exception as e:
        logger.error(f"WS to TCP error: {e}")

async def proxy_data(websocket, tcp_socket):
    """Proxy data between WebSocket and TCP socket"""
    # Create two tasks for bidirectional communication
    tcp_to_ws_task = asyncio.create_task(tcp_to_ws(tcp_socket, websocket))
    ws_to_tcp_task = asyncio.create_task(ws_to_tcp(websocket, tcp_socket))
    
    # Wait for either task to complete
    done, pending = await asyncio.wait(
        [tcp_to_ws_task, ws_to_tcp_task],
        return_when=asyncio.FIRST_COMPLETED
    )
    
    # Cancel the other task
    for task in pending:
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

async def handle_websocket(websocket, path):
    """Handle a new WebSocket connection"""
    logger.info(f"New WebSocket connection from {websocket.remote_address}, path: {path}")
    
    # Parse target host and port from path
    # Expected format: /host:port or use defaults
    target_host = SPICE_HOST
    target_port = SPICE_PORT
    
    if path and path != '/':
        try:
            # Remove leading slash
            target = path.lstrip('/')
            if ':' in target:
                target_host, port_str = target.split(':', 1)
                target_port = int(port_str)
            else:
                # Just host provided, use default port
                target_host = target
        except Exception as e:
            logger.error(f"Failed to parse target from path {path}: {e}")
    
    logger.info(f"Connecting to SPICE server at {target_host}:{target_port}")
    
    # Create TCP connection to SPICE server
    tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    tcp_socket.setblocking(True)
    
    try:
        tcp_socket.connect((target_host, target_port))
        logger.info(f"Connected to SPICE server at {target_host}:{target_port}")
        
        # Start proxying data
        await proxy_data(websocket, tcp_socket)
        
    except Exception as e:
        logger.error(f"Failed to connect to SPICE server: {e}")
        await websocket.close(code=1001, reason=f"Failed to connect to {target_host}:{target_port}")
    finally:
        tcp_socket.close()
        logger.info("Closed TCP connection")

async def main():
    logger.info(f"Starting WebSocket proxy on port {WS_PORT}")
    logger.info(f"Proxying to SPICE server at {SPICE_HOST}:{SPICE_PORT}")
    
    async with websockets.serve(handle_websocket, "0.0.0.0", WS_PORT):
        await asyncio.Future()  # Run forever

if __name__ == "__main__":
    asyncio.run(main())