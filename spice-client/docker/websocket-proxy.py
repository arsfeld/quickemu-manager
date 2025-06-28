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

async def proxy_data(websocket, tcp_socket):
    """Proxy data between WebSocket and TCP socket"""
    try:
        while True:
            # Read from WebSocket and write to TCP
            ws_task = asyncio.create_task(websocket.recv())
            
            # Read from TCP and write to WebSocket
            loop = asyncio.get_event_loop()
            tcp_task = asyncio.create_task(
                loop.run_in_executor(None, tcp_socket.recv, 4096)
            )
            
            # Wait for either to have data
            done, pending = await asyncio.wait(
                [ws_task, tcp_task],
                return_when=asyncio.FIRST_COMPLETED
            )
            
            for task in done:
                if task == ws_task:
                    data = await task
                    if isinstance(data, str):
                        data = data.encode()
                    tcp_socket.sendall(data)
                    logger.debug(f"WS -> TCP: {len(data)} bytes")
                else:
                    data = await task
                    if data:
                        await websocket.send(data)
                        logger.debug(f"TCP -> WS: {len(data)} bytes")
                    else:
                        # TCP socket closed
                        logger.info("TCP socket closed")
                        return
            
            # Cancel pending tasks
            for task in pending:
                task.cancel()
                
    except websockets.exceptions.ConnectionClosed:
        logger.info("WebSocket connection closed")
    except Exception as e:
        logger.error(f"Proxy error: {e}")

async def handle_websocket(websocket):
    """Handle a new WebSocket connection"""
    logger.info(f"New WebSocket connection from {websocket.remote_address}")
    
    # Create TCP connection to SPICE server
    tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    tcp_socket.setblocking(True)
    
    try:
        tcp_socket.connect((SPICE_HOST, SPICE_PORT))
        logger.info(f"Connected to SPICE server at {SPICE_HOST}:{SPICE_PORT}")
        
        # Start proxying data
        await proxy_data(websocket, tcp_socket)
        
    except Exception as e:
        logger.error(f"Failed to connect to SPICE server: {e}")
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