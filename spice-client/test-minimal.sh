#!/bin/bash
# Minimal test to debug connection issues

cd docker
echo "Starting debug server..."
docker compose -f docker-compose.e2e-debug.yml up -d spice-debug-test-server

echo "Waiting for server to start..."
sleep 5

echo "Server logs:"
docker logs spice-debug-test-e2e --tail 20

echo "Testing connection with netcat..."
echo -e '\x52\x45\x44\x51' | nc -w 1 localhost 5912 | xxd

echo "Stopping server..."
docker compose -f docker-compose.e2e-debug.yml down -v