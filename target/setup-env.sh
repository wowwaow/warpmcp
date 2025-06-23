#!/bin/bash

# MCP Server Environment Variables
export TRELLO_KEY="9261212035c0fc6ae135fdc8dbba4b4d"
export TRELLO_TOKEN="ATTA32b292cf73fa669b9279162407f29eee755db2bc3b1a32a78bf7c9597362cf54804EFD24"
export TRELLO_BOARD_ID="2mK8SuCX"
export REDIS_URL="redis://127.0.0.1:6379"
export RUST_LOG_LEVEL="info"
export HEARTBEAT_TIMEOUT="120"

# Additional MCP Configuration
export MCP_ENV="development"
export MCP_VERSION="0.1.0"
export MCP_HOST="127.0.0.1"
export MCP_PORT="8080"

echo "MCP environment variables have been set"
