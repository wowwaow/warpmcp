#!/bin/bash

cd /home/clara-sorrenti/Documents/MF_MCP || exit 1

# Update remote URL to HTTPS
git remote set-url origin https://github.com/wowwaow/warpmcp.git

# Configure git to use GitHub CLI credentials
git config --global credential.helper 'gh auth git-credential'

# Test connection
echo "Testing connection..."
git fetch origin

if [ $? -eq 0 ]; then
    echo "✓ Connection successful!"
    echo "✓ Running sync script..."
    /home/clara-sorrenti/Documents/git-sync.sh
else
    echo "✗ Connection failed. Please run 'gh auth login' to authenticate"
fi
