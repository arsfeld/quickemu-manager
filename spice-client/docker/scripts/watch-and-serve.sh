#!/bin/bash
set -e

# Make the hot-reload server executable
chmod +x /usr/local/bin/hot-reload-server.py

# Run the Python hot-reload server
exec python3 /usr/local/bin/hot-reload-server.py