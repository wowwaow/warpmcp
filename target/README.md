# Warp Terminal Server Integration

This directory contains the configuration for integrating the MCP project with Warp Terminal Server.

## Configuration Files

### `warp.yaml`
Main configuration file for the Warp Terminal Server integration:
- Server settings (port, TLS, auth)
- GitHub integration
- Automated tasks
- Logging configuration
- Monitoring settings
- Development environment settings

### `workflows.yaml`
Defines automated workflows for common tasks:
- Git synchronization
- Development environment setup
- Deployment procedures
- Backup operations
- System monitoring

## Setup

1. **Environment Variables**
   ```bash
   export WARP_AUTH_TOKEN="your-secure-token"
   ```

2. **Directory Structure**
   ```
   target/
   ├── certs/          # TLS certificates
   ├── logs/           # Log files
   ├── backups/        # Backup archives
   ├── warp.yaml       # Main configuration
   ├── workflows.yaml  # Workflow definitions
   └── README.md      # This file
   ```

3. **Create Required Directories**
   ```bash
   mkdir -p certs logs backups
   ```

4. **Generate TLS Certificates**
   ```bash
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout certs/server.key \
     -out certs/server.crt
   ```

## Usage

### Start Warp Server
```bash
warp server start
```

### Run Workflows
```bash
warp workflow run sync    # Run git sync workflow
warp workflow run dev     # Start development environment
warp workflow run deploy  # Deploy application
warp workflow run backup  # Create backup
warp workflow run monitor # Start monitoring
```

### Monitor Status
```bash
warp status              # Check server status
warp logs               # View logs
warp metrics           # View metrics
```

## Security

- TLS encryption enabled by default
- Token-based authentication
- Secure credential handling
- No sensitive data in configuration files

## Maintenance

### Logs
- Located in `logs/` directory
- Rotated automatically (max 100MB per file)
- Retained for 7 days

### Backups
- Created every 6 hours
- Stored in `backups/` directory
- Older than 7 days automatically removed

### Monitoring
- Health checks every minute
- Metrics available on port 9090
- Disk space and log size monitoring

## Troubleshooting

1. **Server Won't Start**
   - Check if port 2025 is available
   - Verify TLS certificates exist
   - Ensure WARP_AUTH_TOKEN is set

2. **Workflow Failures**
   - Check logs in `logs/warp.log`
   - Verify required commands are installed
   - Check file permissions

3. **Sync Issues**
   - Verify GitHub credentials
   - Check network connectivity
   - Review sync script logs
