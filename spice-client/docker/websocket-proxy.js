const WebSocket = require('ws');
const net = require('net');

const WS_PORT = process.env.WS_PORT || 5959;
const SPICE_HOST = process.env.SPICE_HOST || 'qemu-spice';
const SPICE_PORT = process.env.SPICE_PORT || 5900;

console.log(`Starting WebSocket proxy on port ${WS_PORT}`);
console.log(`Forwarding to SPICE server at ${SPICE_HOST}:${SPICE_PORT}`);

const wss = new WebSocket.Server({ port: WS_PORT });

wss.on('connection', (ws) => {
  console.log('New WebSocket connection');
  
  // Create TCP connection to SPICE server
  const tcp = net.createConnection(SPICE_PORT, SPICE_HOST, () => {
    console.log('Connected to SPICE server');
  });
  
  // Forward WebSocket data to TCP
  ws.on('message', (data) => {
    if (tcp.readyState === 'open') {
      tcp.write(data);
    }
  });
  
  // Forward TCP data to WebSocket
  tcp.on('data', (data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(data);
    }
  });
  
  // Handle errors and cleanup
  ws.on('close', () => {
    console.log('WebSocket closed');
    tcp.end();
  });
  
  ws.on('error', (err) => {
    console.error('WebSocket error:', err);
    tcp.end();
  });
  
  tcp.on('close', () => {
    console.log('TCP connection closed');
    ws.close();
  });
  
  tcp.on('error', (err) => {
    console.error('TCP error:', err);
    ws.close();
  });
});

console.log(`WebSocket proxy listening on port ${WS_PORT}`);