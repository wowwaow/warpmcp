#!/bin/bash

# Setup script for bidirectional Git sync
# Run this once to configure everything

SCRIPT_DIR="/home/clara-sorrenti/Documents"
SYNC_SCRIPT="$SCRIPT_DIR/git-sync.sh"
LOCAL_REPO="/home/clara-sorrenti/Documents/MF_MCP"

echo "Setting up bidirectional Git sync..."

# Create the sync script
echo "Creating sync script at $SYNC_SCRIPT..."
cat > "$SYNC_SCRIPT" << 'EOF'
#!/bin/bash

# Bidirectional Git Sync Script
# Syncs local folder with GitHub repo every 10 minutes

# Configuration
LOCAL_REPO="/home/clara-sorrenti/Documents/MF_MCP"
REMOTE_URL="https://github.com/wowwaow/warpmcp.git"
BRANCH="main"
LOG_FILE="/home/clara-sorrenti/Documents/git-sync.log"

# Function to log messages with timestamp
log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Function to check if directory is a git repository
is_git_repo() {
    git -C "$1" rev-parse --git-dir > /dev/null 2>&1
}

# Function to setup repository if it doesn't exist
setup_repo() {
    log_message "Setting up repository..."
    
    if [ ! -d "$LOCAL_REPO" ]; then
        log_message "Creating directory: $LOCAL_REPO"
        mkdir -p "$LOCAL_REPO"
    fi
    
    cd "$LOCAL_REPO"
    
    if ! is_git_repo "$LOCAL_REPO"; then
        log_message "Initializing git repository and adding remote..."
        git init
        git remote add origin "$REMOTE_URL"
        git fetch origin
        
        # Check if remote has content
        if git ls-remote --heads origin | grep -q "$BRANCH"; then
            log_message "Remote branch exists, checking out..."
            git checkout -b "$BRANCH" "origin/$BRANCH"
        else
            log_message "Remote branch doesn't exist, creating initial commit..."
            git checkout -b "$BRANCH"
            echo "# MF_MCP" > README.md
            git add README.md
            git commit -m "Initial commit"
            git push -u origin "$BRANCH"
        fi
    else
        log_message "Git repository already exists"
    fi
}

# Function to perform bidirectional sync
sync_repo() {
    cd "$LOCAL_REPO"
    
    # Check if we're in a git repository
    if ! is_git_repo "$LOCAL_REPO"; then
        log_message "ERROR: Not a git repository. Running setup..."
        setup_repo
        return
    fi
    
    log_message "Starting sync process..."
    
    # Stash any uncommitted changes temporarily
    local stash_created=false
    if [[ -n $(git status --porcelain) ]]; then
        log_message "Stashing local changes..."
        git stash push -m "Auto-stash before sync $(date)"
        stash_created=true
    fi
    
    # Fetch latest changes from remote
    log_message "Fetching from remote..."
    if ! git fetch origin; then
        log_message "ERROR: Failed to fetch from remote"
        return 1
    fi
    
    # Check if remote has new commits
    local local_commit=$(git rev-parse HEAD)
    local remote_commit=$(git rev-parse origin/$BRANCH)
    
    if [ "$local_commit" != "$remote_commit" ]; then
        log_message "Remote has new changes, pulling..."
        if ! git pull origin "$BRANCH"; then
            log_message "ERROR: Failed to pull changes"
            if [ "$stash_created" = true ]; then
                log_message "Restoring stashed changes..."
                git stash pop
            fi
            return 1
        fi
        log_message "Successfully pulled remote changes"
    else
        log_message "No new remote changes"
    fi
    
    # Restore stashed changes if any
    if [ "$stash_created" = true ]; then
        log_message "Restoring local changes..."
        if ! git stash pop; then
            log_message "WARNING: Conflict while restoring local changes. Manual intervention may be required."
        fi
    fi
    
    # Check for local changes to commit and push
    if [[ -n $(git status --porcelain) ]]; then
        log_message "Local changes detected, committing..."
        git add .
        
        # Create commit message with timestamp
        local commit_msg="Auto-sync: $(date '+%Y-%m-%d %H:%M:%S')"
        
        if git commit -m "$commit_msg"; then
            log_message "Changes committed successfully"
            
            # Push to remote
            if git push origin "$BRANCH"; then
                log_message "Changes pushed to remote successfully"
            else
                log_message "ERROR: Failed to push changes to remote"
                return 1
            fi
        else
            log_message "ERROR: Failed to commit changes"
            return 1
        fi
    else
        log_message "No local changes to commit"
    fi
    
    log_message "Sync completed successfully"
}

# Main execution
main() {
    log_message "=== Git Sync Started ==="
    
    # Setup repository if needed
    if [ ! -d "$LOCAL_REPO" ] || ! is_git_repo "$LOCAL_REPO"; then
        setup_repo
    fi
    
    # Perform sync
    sync_repo
    
    log_message "=== Git Sync Finished ==="
    echo "" >> "$LOG_FILE"  # Add blank line for readability
}

# Run main function
main "$@"
EOF

# Make the sync script executable
chmod +x "$SYNC_SCRIPT"
echo "Sync script created and made executable"

# Setup cron job for every 10 minutes
echo "Setting up cron job for every 10 minutes..."

# Create a temporary cron file
TEMP_CRON=$(mktemp)

# Get existing cron jobs
crontab -l 2>/dev/null > "$TEMP_CRON"

# Check if our cron job already exists
if ! grep -q "$SYNC_SCRIPT" "$TEMP_CRON"; then
    # Add our cron job (every 10 minutes)
    echo "*/10 * * * * $SYNC_SCRIPT >> /home/clara-sorrenti/Documents/git-sync-cron.log 2>&1" >> "$TEMP_CRON"
    
    # Install the new cron file
    crontab "$TEMP_CRON"
    echo "Cron job added successfully"
else
    echo "Cron job already exists"
fi

# Clean up temporary file
rm "$TEMP_CRON"

# Check Git configuration
echo ""
echo "Checking Git configuration..."
if ! git config --global user.name > /dev/null; then
    echo "WARNING: Git user.name not set. Please run:"
    echo "  git config --global user.name 'Your Name'"
fi

if ! git config --global user.email > /dev/null; then
    echo "WARNING: Git user.email not set. Please run:"
    echo "  git config --global user.email 'your.email@example.com'"
fi

# Check GitHub authentication
echo ""
echo "Testing GitHub authentication..."
if git ls-remote https://github.com/wowwaow/warpmcp.git > /dev/null 2>&1; then
    echo "✓ GitHub repository is accessible"
else
    echo "⚠ WARNING: Cannot access GitHub repository"
    echo "You may need to set up authentication:"
    echo "  1. For HTTPS: Set up a personal access token"
    echo "  2. For SSH: Set up SSH keys"
    echo "  3. Run: git config --global credential.helper store"
fi

# Initial setup
echo ""
echo "Running initial repository setup..."
"$SYNC_SCRIPT"

echo ""
echo "Setup complete! The sync will run every 10 minutes."
echo "Logs will be written to: /home/clara-sorrenti/Documents/git-sync.log"
echo "Cron logs will be written to: /home/clara-sorrenti/Documents/git-sync-cron.log"
echo ""
echo "To manually run sync: $SYNC_SCRIPT"
echo "To check cron jobs: crontab -l"
echo "To view sync logs: tail -f /home/clara-sorrenti/Documents/git-sync.log"
