#!/usr/bin/env python3
"""
Multi-channel WebSocket to TCP proxy for SPICE protocol.
Supports proxying multiple SPICE channels through separate WebSocket connections.
"""
import asyncio
import websockets
import socket
import logging
import os
import json
from typing import Dict, Tuple
from urllib.parse import urlparse, parse_qs

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Configuration from environment
WS_BASE_PORT = int(os.environ.get('WS_BASE_PORT', '8080'))
SPICE_HOST = os.environ.get('SPICE_HOST', 'qemu-spice')

# SPICE channel types and their default ports
CHANNEL_PORTS = {
    'main': int(os.environ.get('MAIN_CHANNEL_PORT', '5900')),
    'display': int(os.environ.get('DISPLAY_CHANNEL_PORT', '5901')),
    'inputs': int(os.environ.get('INPUTS_CHANNEL_PORT', '5902')),
    'cursor': int(os.environ.get('CURSOR_CHANNEL_PORT', '5903')),
    'playback': int(os.environ.get('PLAYBACK_CHANNEL_PORT', '5904')),
    'record': int(os.environ.get('RECORD_CHANNEL_PORT', '5905')),
}

class ConnectionStats:
    """Track connection statistics for monitoring"""
    def __init__(self):
        self.bytes_sent = 0
        self.bytes_received = 0
        self.messages_sent = 0
        self.messages_received = 0
        self.start_time = asyncio.get_event_loop().time()
    
    def to_dict(self):
        current_time = asyncio.get_event_loop().time()
        duration = current_time - self.start_time
        return {
            'bytes_sent': self.bytes_sent,
            'bytes_received': self.bytes_received,
            'messages_sent': self.messages_sent,
            'messages_received': self.messages_received,
            'duration_seconds': duration,
            'throughput_mbps': (self.bytes_sent + self.bytes_received) * 8 / (duration * 1_000_000) if duration > 0 else 0
        }

class SpiceWebSocketProxy:
    def __init__(self):
        self.connections: Dict[str, ConnectionStats] = {}
        self.active_proxies = set()
    
    def parse_channel_info(self, path: str) -> Tuple[str, str, int]:
        """
        Parse WebSocket path to determine channel type and target.
        Formats supported:
        - /main
        - /display?host=server&port=5901
        - /inputs/server:5902
        """
        # Remove leading slash
        path = path.lstrip('/')
        
        # Default values
        channel_type = 'main'
        target_host = SPICE_HOST
        target_port = CHANNEL_PORTS.get('main', 5900)
        
        # Parse path components
        if '?' in path:
            # Query string format: /channel?host=server&port=5901
            channel_part, query_string = path.split('?', 1)
            channel_type = channel_part or 'main'
            
            params = parse_qs(query_string)
            if 'host' in params:
                target_host = params['host'][0]
            if 'port' in params:
                target_port = int(params['port'][0])
            else:
                target_port = CHANNEL_PORTS.get(channel_type, 5900)
        elif '/' in path:
            # Path format: /channel/server:port
            parts = path.split('/', 1)
            channel_type = parts[0]
            if len(parts) > 1:
                server_spec = parts[1]
                if ':' in server_spec:
                    target_host, port_str = server_spec.rsplit(':', 1)
                    target_port = int(port_str)
                else:
                    target_host = server_spec
                    target_port = CHANNEL_PORTS.get(channel_type, 5900)
        else:
            # Simple format: /channel
            channel_type = path or 'main'
            target_port = CHANNEL_PORTS.get(channel_type, 5900)
        
        return channel_type, target_host, target_port
    
    async def tcp_to_ws(self, tcp_socket: socket.socket, websocket, stats: ConnectionStats):
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
                stats.bytes_sent += len(data)
                stats.messages_sent += 1
                
                if stats.messages_sent % 100 == 0:
                    logger.debug(f"TCP -> WS: {stats.to_dict()}")
                    
        except Exception as e:
            logger.error(f"TCP to WS error: {e}")
    
    async def ws_to_tcp(self, websocket, tcp_socket: socket.socket, stats: ConnectionStats):
        """Forward data from WebSocket to TCP"""
        try:
            while True:
                data = await websocket.recv()
                if isinstance(data, str):
                    data = data.encode()
                
                tcp_socket.sendall(data)
                stats.bytes_received += len(data)
                stats.messages_received += 1
                
                if stats.messages_received % 100 == 0:
                    logger.debug(f"WS -> TCP: {stats.to_dict()}")
                    
        except websockets.exceptions.ConnectionClosed:
            logger.info("WebSocket closed")
        except Exception as e:
            logger.error(f"WS to TCP error: {e}")
    
    async def proxy_connection(self, websocket, tcp_socket: socket.socket, connection_id: str):
        """Proxy data between WebSocket and TCP socket"""
        stats = ConnectionStats()
        self.connections[connection_id] = stats
        
        try:
            # Create bidirectional proxy tasks
            tcp_to_ws_task = asyncio.create_task(
                self.tcp_to_ws(tcp_socket, websocket, stats)
            )
            ws_to_tcp_task = asyncio.create_task(
                self.ws_to_tcp(websocket, tcp_socket, stats)
            )
            
            # Store tasks for cleanup
            self.active_proxies.add((tcp_to_ws_task, ws_to_tcp_task))
            
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
            
        finally:
            # Cleanup
            self.active_proxies.discard((tcp_to_ws_task, ws_to_tcp_task))
            if connection_id in self.connections:
                final_stats = self.connections[connection_id].to_dict()
                logger.info(f"Connection {connection_id} closed. Stats: {final_stats}")
                del self.connections[connection_id]
    
    async def handle_websocket(self, websocket, path: str):
        """Handle a new WebSocket connection"""
        remote_addr = f"{websocket.remote_address[0]}:{websocket.remote_address[1]}"
        logger.info(f"New WebSocket connection from {remote_addr}, path: {path}")
        
        # Parse channel information from path
        channel_type, target_host, target_port = self.parse_channel_info(path)
        connection_id = f"{remote_addr}-{channel_type}-{target_host}:{target_port}"
        
        logger.info(f"Channel: {channel_type}, Target: {target_host}:{target_port}")
        
        # Create TCP connection to SPICE server
        tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        tcp_socket.setblocking(True)
        
        try:
            # Set socket options for better performance
            tcp_socket.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            tcp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
            
            # Connect with timeout
            tcp_socket.settimeout(10.0)
            tcp_socket.connect((target_host, target_port))
            tcp_socket.settimeout(None)  # Remove timeout after connection
            
            logger.info(f"Connected to SPICE {channel_type} channel at {target_host}:{target_port}")
            
            # Start proxying
            await self.proxy_connection(websocket, tcp_socket, connection_id)
            
        except socket.timeout:
            logger.error(f"Timeout connecting to {target_host}:{target_port}")
            await websocket.close(
                code=1001, 
                reason=f"Connection timeout to {target_host}:{target_port}"
            )
        except Exception as e:
            logger.error(f"Failed to connect to SPICE server: {e}")
            await websocket.close(
                code=1001, 
                reason=f"Failed to connect to {target_host}:{target_port}: {str(e)}"
            )
        finally:
            tcp_socket.close()
            logger.info(f"Closed TCP connection for {connection_id}")
    
    async def handle_stats(self, websocket, path: str):
        """WebSocket endpoint for real-time statistics"""
        if path != '/stats':
            await websocket.close(code=1002, reason="Invalid stats path")
            return
        
        logger.info("Stats WebSocket connected")
        try:
            while True:
                # Send current connection statistics
                stats = {
                    'connections': {
                        conn_id: stats.to_dict() 
                        for conn_id, stats in self.connections.items()
                    },
                    'active_proxies': len(self.active_proxies),
                    'timestamp': asyncio.get_event_loop().time()
                }
                
                await websocket.send(json.dumps(stats))
                await asyncio.sleep(1)  # Update every second
                
        except websockets.exceptions.ConnectionClosed:
            logger.info("Stats WebSocket closed")
    
    async def start_server(self, port: int):
        """Start WebSocket server on specified port"""
        # Create a wrapper that extracts path from the websocket object
        async def handler_wrapper(websocket):
            # Extract path from the WebSocket connection
            path = websocket.path if hasattr(websocket, 'path') else '/'
            
            # Determine which handler to use based on port
            if port == WS_BASE_PORT + 999:  # Stats port
                await self.handle_stats(websocket, path)
            else:
                await self.handle_websocket(websocket, path)
        
        logger.info(f"Starting WebSocket server on port {port}")
        async with websockets.serve(handler_wrapper, "0.0.0.0", port):
            await asyncio.Future()  # Run forever

async def main():
    """Start multi-channel proxy servers"""
    proxy = SpiceWebSocketProxy()
    
    # Start a server for each channel type
    servers = []
    
    # Main channels (one per channel type)
    for i, channel_type in enumerate(CHANNEL_PORTS.keys()):
        port = WS_BASE_PORT + i
        logger.info(f"Starting {channel_type} channel proxy on port {port}")
        server_task = asyncio.create_task(proxy.start_server(port))
        servers.append(server_task)
    
    # Stats server
    stats_port = WS_BASE_PORT + 999
    logger.info(f"Starting stats server on port {stats_port}")
    stats_task = asyncio.create_task(proxy.start_server(stats_port))
    servers.append(stats_task)
    
    # Log configuration
    logger.info("WebSocket proxy configuration:")
    logger.info(f"  SPICE host: {SPICE_HOST}")
    for channel, port in CHANNEL_PORTS.items():
        logger.info(f"  {channel}: TCP port {port} -> WS port {WS_BASE_PORT + list(CHANNEL_PORTS.keys()).index(channel)}")
    logger.info(f"  Stats available at: ws://localhost:{stats_port}/stats")
    
    # Wait for all servers
    try:
        await asyncio.gather(*servers)
    except KeyboardInterrupt:
        logger.info("Shutting down proxy servers...")
        for task in servers:
            task.cancel()

if __name__ == "__main__":
    asyncio.run(main())