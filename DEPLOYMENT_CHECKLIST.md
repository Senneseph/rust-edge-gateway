# DigitalOcean Deployment Checklist

This checklist guides you through deploying the Rust Edge Gateway with SQLite to your DigitalOcean droplet.

## Server Information

- **IP Address:** 167.71.191.234 (from .env file)
- **Domain:** rust-edge-gateway.iffuso.com (from .env file)
- **SSH Key:** .ssh/a-icon-deploy (from .env file)

## Pre-Deployment

- [ ] Verify SSH key exists: `Test-Path $env:USERPROFILE\.ssh\a-icon-deploy`
- [ ] Verify Docker is installed on droplet: `ssh -i $key root@167.71.191.234 "docker --version"`
- [ ] Verify Docker Compose is installed: `ssh -i $key root@167.71.191.234 "docker-compose --version"`
- [ ] Review environment variables in `.env`
- [ ] Back up any existing database on the droplet

## Step 1: Connect to Droplet

```powershell
$key = "$env:USERPROFILE\.ssh\a-icon-deploy"
$server = "167.71.191.234"

ssh -i $key root@$server
```

## Step 2: Setup Project Directory

```bash
# Create project directory
mkdir -p /opt/rust-edge-gateway
cd /opt/rust-edge-gateway

# Clone repository (or sync if already cloned)
git clone https://github.com/Senneseph/Rust-Edge-Gateway.git .
# OR if already cloned:
git pull origin master

# Create data directory for persistence
mkdir -p data
mkdir -p handlers
```

- [ ] Project directory created
- [ ] Git repository synced
- [ ] Directories created

## Step 3: Configure Environment

```bash
# Create/verify .env file
cat > .env << 'EOF'
DIGITALOCEAN_ACCESS_TOKEN=dop_v1_...
DEPLOY_SERVER_IP=167.71.191.234
TARGET_DOMAIN=rust-edge-gateway.iffuso.com
DOCS_DOMAIN=docs.rust-edge-gateway.iffuso.com
SSH_KEY=$env:USERPROFILE\.ssh\a-icon-deploy
EOF

# Verify configuration
cat .env
```

- [ ] .env file created/updated
- [ ] All variables are correct

## Step 4: Build Handler Image

```bash
# Build the Docker image
docker build -t rust-edge-gateway:latest .

# Verify build completed
docker images | grep rust-edge-gateway
```

- [ ] Docker image built successfully

## Step 5: Start Services

```bash
# Copy production compose file
cp docker-compose.prod.yml docker-compose.yml

# Start all services
docker-compose up -d

# Verify services started
docker-compose ps
```

Expected output:
- `live-sqlite` - Running ✓
- `rust-edge-gateway` - Running ✓
- `rust-edge-gateway-caddy` - Running ✓

- [ ] Services started
- [ ] All containers running

## Step 6: Health Checks

```bash
# Check gateway health
curl -s http://localhost:8081/api/health | jq .

# Check SQLite health
curl -s http://localhost:8282/health

# Check Caddy (reverse proxy)
curl -s -I http://localhost:80/ | head -5

# View logs
docker-compose logs --tail=50 -f
```

- [ ] Gateway responds to health check
- [ ] SQLite service is healthy
- [ ] Reverse proxy is running

## Step 7: Configure SSL/TLS

Edit `Caddyfile` for your domain:

```caddyfile
rust-edge-gateway.iffuso.com {
    reverse_proxy rust-edge-gateway:8080
    
    # Auto-renew Let's Encrypt certificates
    encode gzip
    
    # Security headers
    header / -Server
    header / X-Content-Type-Options nosniff
    header / X-Frame-Options DENY
}
```

Reload Caddy:
```bash
docker exec rust-edge-gateway-caddy caddy reload --config /etc/caddy/Caddyfile
```

- [ ] Caddyfile configured
- [ ] SSL certificates auto-renewed
- [ ] HTTPS accessible

## Step 8: Test API Access

```bash
# External test (from local machine)
$server = "167.71.191.234"

# Test HTTP (should redirect to HTTPS)
curl -i http://$server:8080/api/health

# Test HTTPS
curl -s https://rust-edge-gateway.iffuso.com/api/health | jq .

# Test SQLite (internal only during development)
# SSH and test locally: curl http://localhost:8282/health
```

- [ ] HTTP port accessible
- [ ] HTTPS redirects working
- [ ] API responding

## Step 9: Create Initial Database Schema

```bash
# SSH into gateway container
docker exec -it rust-edge-gateway bash

# Create test table
curl -X POST http://live-sqlite:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, data TEXT)",
    "params": []
  }'
```

- [ ] Database schema created
- [ ] Test table created

## Step 10: Deploy Test Handler

```bash
# Build test handler
cd handlers/a-icon-sqlite-test
cargo build --release
cd ../../

# Verify binary created
ls -la handlers/a-icon-sqlite-test/target/release/handler_*
```

- [ ] Test handler built
- [ ] Binary exists

## Step 11: Register Handler via Admin UI

1. Open `https://rust-edge-gateway.iffuso.com:8081/admin/`
2. Create Domain: `a-icon.local`
3. Create Collection: `tests`
4. Create Endpoint:
   - Name: `sqlite-test`
   - Path: `/sqlite-test`
   - Method: `GET`
   - Handler: Select the built handler

Or via API:
```bash
curl -X POST https://rust-edge-gateway.iffuso.com:8081/api/handlers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sqlite-test",
    "path": "/sqlite-test",
    "domain_id": "a-icon-local",
    "method": "GET"
  }'
```

- [ ] Domain created
- [ ] Collection created
- [ ] Handler registered

## Step 12: Test Handler

```bash
# From your local machine
$domain = "rust-edge-gateway.iffuso.com"

# Test handler health
curl -s https://$domain/sqlite-test/health | jq .

# Test SQLite connection
curl -s https://$domain/sqlite-test/test-connection | jq .

# Create test table
curl -s https://$domain/sqlite-test/create-table | jq .

# Insert data
curl -s https://$domain/sqlite-test/insert-data | jq .

# Query data
curl -X POST https://$domain/sqlite-test/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM test_table;"}' | jq .
```

- [ ] Handler responds
- [ ] SQLite connection working
- [ ] Data persistence verified

## Step 13: Backup Setup

```bash
# Create backup directory
mkdir -p /opt/backups

# Create backup script
cat > /opt/backups/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/opt/backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Backup SQLite database
docker run --rm \
  -v sqlite_data:/data \
  -v $BACKUP_DIR:/backup \
  alpine cp /data/app.db /backup/app.db.$TIMESTAMP

# Keep only last 7 backups
find $BACKUP_DIR -name "app.db.*" -mtime +7 -delete

echo "Backup completed: app.db.$TIMESTAMP"
EOF

# Make executable
chmod +x /opt/backups/backup.sh

# Add to crontab for daily backups at 2 AM
(crontab -l 2>/dev/null; echo "0 2 * * * /opt/backups/backup.sh") | crontab -

# Verify crontab
crontab -l | grep backup.sh
```

- [ ] Backup directory created
- [ ] Backup script created
- [ ] Cron job configured

## Step 14: Monitoring Setup

```bash
# Create monitoring script
cat > /opt/check-health.sh << 'EOF'
#!/bin/bash

echo "=== Docker Container Status ==="
docker-compose ps

echo -e "\n=== Gateway Health ==="
curl -s http://localhost:8081/api/health || echo "FAILED"

echo -e "\n=== SQLite Health ==="
curl -s http://localhost:8282/health || echo "FAILED"

echo -e "\n=== Disk Usage ==="
df -h /opt/rust-edge-gateway

echo -e "\n=== Memory Usage ==="
docker stats --no-stream
EOF

chmod +x /opt/check-health.sh
```

- [ ] Monitoring script created

## Verification Checklist

```bash
# Run from droplet
/opt/check-health.sh

# Expected output:
# - All containers RUNNING
# - Gateway returns health status
# - SQLite returns 200 OK
# - Adequate disk and memory
```

- [ ] All checks passing

## Post-Deployment

### Access Points

- **Admin UI:** `https://rust-edge-gateway.iffuso.com:8081/admin/`
- **Gateway API:** `https://rust-edge-gateway.iffuso.com/`
- **SQLite:** Internal only (port 8282 on droplet)

### Updating Handlers

To deploy a new or updated handler:

```bash
# On your local machine
cd handlers/your-handler
cargo build --release

# Copy to droplet
scp -i $key -r target/release/handler_* root@167.71.191.234:/opt/rust-edge-gateway/handlers/your-handler/target/release/

# Restart gateway (optional, auto-detects new binaries)
ssh -i $key root@167.71.191.234 "docker-compose -C /opt/rust-edge-gateway restart rust-edge-gateway"
```

### Database Management

```bash
# SSH into droplet
ssh -i $key root@167.71.191.234

# Connect to SQLite directly
docker exec -it live-sqlite sqlite3 /data/app.db

# Backup database
/opt/backups/backup.sh

# View backups
ls -lah /opt/backups/
```

### Logs

```bash
# View recent logs
docker-compose logs --tail=50 rust-edge-gateway
docker-compose logs --tail=50 live-sqlite

# Follow logs in real-time
docker-compose logs -f

# Save logs to file
docker-compose logs > /tmp/gateway-logs.txt
```

## Troubleshooting

### Services Won't Start

```bash
# Check for port conflicts
netstat -tulpn | grep 8080

# Check Docker logs
docker-compose logs --tail=100

# Rebuild and restart
docker-compose down -v  # WARNING: Removes volumes!
docker-compose up -d
```

### SQLite Connection Issues

```bash
# Check if SQLite container is healthy
docker ps | grep live-sqlite

# Test health
docker exec live-sqlite curl http://localhost:8080/health

# Check volume
docker volume inspect sqlite_data

# Check database file permissions
docker exec -it live-sqlite ls -la /data/
```

### Handler Not Found

```bash
# Check handler binary exists
ls -la /opt/rust-edge-gateway/handlers/*/target/release/handler_*

# Rebuild handler
cd /opt/rust-edge-gateway/handlers/your-handler
cargo build --release

# Restart gateway
docker-compose restart rust-edge-gateway
```

## Rollback Procedure

If something goes wrong:

```bash
# Stop services
docker-compose stop

# Restore database from backup
BACKUP_FILE=$(ls /opt/backups/app.db.* | sort -r | head -1)
docker run --rm \
  -v sqlite_data:/data \
  -v /opt/backups:/backups \
  alpine cp /backups/$(basename $BACKUP_FILE) /data/app.db

# Restart services
docker-compose up -d

# Verify
docker-compose ps
curl http://localhost:8081/api/health
```

## Completion

- [ ] All services running and healthy
- [ ] Handlers registered and responding
- [ ] SQLite persisting data
- [ ] Backups configured
- [ ] Monitoring in place
- [ ] Logs accessible
- [ ] SSL/TLS working

**Deployment Complete!**

For ongoing management, refer to:
- SQLITE_SETUP_GUIDE.md
- SQLITE_QUICK_START.md
- IMPLEMENTATION_SUMMARY.md
