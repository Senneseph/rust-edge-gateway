# Fix .env file on remote droplet and restart container
# This fixes the reCAPTCHA typo without full redeployment

$ErrorActionPreference = "Stop"

# Load .env file
if (Test-Path .env) {
    Get-Content .env | ForEach-Object {
        if ($_ -match '^([^#][^=]+)=(.*)$') {
            $name = $matches[1].Trim()
            $value = $matches[2].Trim()
            Set-Item -Path "env:$name" -Value $value
        }
    }
}

$SSH_KEY_PATH = $ExecutionContext.InvokeCommand.ExpandString($env:SSH_KEY)

Write-Host "=== Fixing .env on Droplet ===" -ForegroundColor Green
Write-Host "Server: $env:DEPLOY_SERVER_IP"
Write-Host ""

# Step 1: Copy corrected .env file
Write-Host "Step 1: Copying corrected .env file..." -ForegroundColor Cyan
scp -i $SSH_KEY_PATH .env root@$($env:DEPLOY_SERVER_IP):/opt/rust-edge-gateway/.env

# Step 2: Restart container to pick up new env vars
Write-Host ""
Write-Host "Step 2: Restarting container..." -ForegroundColor Cyan

$restartCommands = @"
cd /opt/rust-edge-gateway
echo 'Stopping container...'
docker-compose down
echo 'Starting container with new environment...'
docker-compose up -d
sleep 5
echo 'Container status:'
docker-compose ps
echo ''
echo 'Recent logs:'
docker-compose logs --tail 20
"@

ssh -i $SSH_KEY_PATH root@$env:DEPLOY_SERVER_IP $restartCommands

Write-Host ""
Write-Host "=== Fix Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "The reCAPTCHA configuration has been fixed."
Write-Host "You should now be able to login at: https://$env:TARGET_DOMAIN/admin/"

