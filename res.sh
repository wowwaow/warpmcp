#!/bin/bash
cd /home/clara-sorrenti/Documents/MF_MCP

echo "=== Fixing all sync issues ==="

# Fix permissions
sudo chown -R clara-sorrenti:clara-sorrenti /home/clara-sorrenti/Documents/
chmod 644 /home/clara-sorrenti/Documents/git-sync.log

# Configure git
git config user.name "Clara Sorrenti"
git config user.email "clara@example.com"  # Replace with your email
git config pull.rebase false

# Clean up build artifacts
echo -e "\n# Rust build artifacts\nMCP/target/\n*.log\n.DS_Store" >> .gitignore
git rm -r --cached MCP/target/ 2>/dev/null || true
git reset HEAD MCP/target/ 2>/dev/null || true

# Remove problematic files
rm -f sync.log fix.sh 2>/dev/null || true

# Resolve README.md conflict - use remote version
git checkout --theirs README.md 2>/dev/null || true

# Add resolved files
git add .gitignore
git add README.md
git add setup.sh 2>/dev/null || true

# Commit
git commit -m "Fix permissions, clean artifacts, resolve conflicts"

# Push
git push origin main

echo "=== Testing sync ==="
# Test sync (as user, not root)
/home/clara-sorrenti/Documents/git-sync.sh
