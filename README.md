# AndyCoin Bot

AndyCoin Bot does one thing, let you give people in your server AndyCoin. It also does its best to cause chaos, indirectly.

## Commands

- `/give` - Give AndyCoins to a user
- `/balance` - Check your AndyCoin balance or another user's balance
- `/leaderboard` - See the server or global leaderboard for AndyCoin
- `/flip` - Flip an AndyCoin, optionally guess heads or tails and gamble
- `/config` - Configure the giver role for giving AndyCoins
- `/vote` - Start a vote to reset all AndyCoins in the server or cast your vote
  - Options: "Start a new vote", "Vote yes", or "Vote no"
- `/vote_admin` - Administrative commands for vote management
  - `/vote_admin status` - Check the status of the current vote
  - `/vote_admin config` - Configure vote settings (cooldown, duration, etc.)
  - `/vote_admin end` - Force end the current vote (admin only)

## Deployment Options

AndyCoin Bot can be deployed in various ways:

### Docker

The simplest way to run AndyCoin Bot is using Docker:

```bash
# Build the Docker image
docker build -t andy-coin:latest .

# Run the container
docker run -d \
  --name andy-coin \
  -e DISCORD_TOKEN=your_discord_token \
  -e RUST_LOG=info \
  -v ./data:/app/data \
  -v ./logs:/app/logs \
  andy-coin:latest
```

### Docker Compose

For a more managed deployment, use Docker Compose:

```bash
# Create a .env file with your Discord token
echo "DISCORD_TOKEN=your_discord_token" > .env

# Start the service
docker-compose up -d
```

### Kubernetes

To deploy on Kubernetes:

1. Update the secret with your Discord token:

```bash
# Create a base64 encoded token
TOKEN_BASE64=$(echo -n "your_discord_token" | base64)

# Update the secret.yaml file
sed -i "s/RElTQ09SRF9UT0tFTl9IRVJF/$TOKEN_BASE64/" kubernetes/secret.yaml

# Apply the Kubernetes manifests
kubectl apply -k kubernetes/
```

### AWS ECS

To deploy on AWS ECS:

1. Create an ECR repository:

```bash
aws ecr create-repository --repository-name andy-coin
```

2. Build and push the Docker image:

```bash
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin your-account-id.dkr.ecr.us-east-1.amazonaws.com
docker build -t your-account-id.dkr.ecr.us-east-1.amazonaws.com/andy-coin:latest .
docker push your-account-id.dkr.ecr.us-east-1.amazonaws.com/andy-coin:latest
```

3. Create an EFS file system for persistent storage:

```bash
aws efs create-file-system --performance-mode generalPurpose --throughput-mode bursting --tags Key=Name,Value=andy-coin-data
```

4. Store your Discord token in AWS Parameter Store:

```bash
aws ssm put-parameter --name /andy-coin/discord-token --type SecureString --value "your_discord_token"
```

5. Deploy using CloudFormation:

```bash
aws cloudformation deploy \
  --template-file aws/cloudformation.yaml \
  --stack-name andy-coin \
  --parameter-overrides \
    VpcId=vpc-xxxxxxxx \
    SubnetIds=subnet-xxxxxxxx,subnet-yyyyyyyy \
    EfsFileSystemId=fs-xxxxxxxx
```

### Azure

To deploy on Azure:

1. Create an Azure Container Registry:

```bash
az acr create --resource-group myResourceGroup --name myRegistry --sku Basic
```

2. Build and push the Docker image:

```bash
az acr login --name myRegistry
docker build -t myregistry.azurecr.io/andy-coin:latest .
docker push myregistry.azurecr.io/andy-coin:latest
```

3. Create a storage account and file share:

```bash
az storage account create --name mystorageaccount --resource-group myResourceGroup --location eastus --sku Standard_LRS
az storage share create --name andy-coin-data --account-name mystorageaccount
```

4. Deploy using ARM template:

```bash
az deployment group create \
  --resource-group myResourceGroup \
  --template-file azure/app-service.json \
  --parameters \
    discordToken=your_discord_token \
    storageAccountName=mystorageaccount \
    storageAccountKey=$(az storage account keys list --resource-group myResourceGroup --account-name mystorageaccount --query "[0].value" -o tsv) \
    registryUsername=$(az acr credential show --name myRegistry --query "username" -o tsv) \
    registryPassword=$(az acr credential show --name myRegistry --query "passwords[0].value" -o tsv) \
    imageName=myregistry.azurecr.io/andy-coin:latest
```

### Cloudflare

AndyCoin Bot can be monitored and backed up using Cloudflare Workers:

1. Create a D1 database:

```bash
cd cloudflare
wrangler d1 create andy_coin_db
```

2. Create a KV namespace:

```bash
wrangler kv:namespace create ANDY_COIN_KV
```

3. Update the wrangler.toml file with your database ID and KV namespace ID.

4. Deploy the worker:

```bash
wrangler deploy
```

## AndyCoin Bot Logging and Auditing

This document describes the logging and auditing system implemented for the AndyCoin Discord bot.

## Logging System

The AndyCoin bot uses the `tracing` ecosystem for structured logging. This provides:

- Detailed logs of all commands executed
- Audit trail of all balance changes
- Structured JSON output for easy parsing
- Daily log rotation

### Log Files

Logs are stored in the `logs` directory with the following files:

- `commands-YYYY-MM-DD.log` - Records all commands executed by users
- `balances-YYYY-MM-DD.log` - Records all balance changes

### Log Format

Logs are stored in JSON format with the following structure:

#### Command Logs

```json
{
  "timestamp": "2025-03-11T18:30:45.123456Z",
  "level": "INFO",
  "target": "command",
  "fields": {
    "command": "give",
    "guild_id": "123456789012345678",
    "user_id": "987654321098765432",
    "arguments": "amount: 50, user: SomeUser#1234",
    "result": "success",
    "message": "Command executed successfully"
  }
}
```

#### Balance Logs

```json
{
  "timestamp": "2025-03-11T18:30:45.123456Z",
  "level": "INFO",
  "target": "balance",
  "fields": {
    "guild_id": "123456789012345678",
    "user_id": "987654321098765432",
    "previous_balance": 100,
    "new_balance": 150,
    "change": 50,
    "reason": "give_command",
    "initiator": "123456789012345678",
    "message": "Balance changed"
  }
}
```

## Audit Tool

The bot includes an audit tool to help analyze logs and track balance changes. The tool is available as a binary in `src/bin/audit.rs`.

### Building the Audit Tool

```bash
cargo build --bin audit
```

### Using the Audit Tool

```bash
# List all commands executed by a specific user
cargo run --bin audit user-commands 123456789012345678

# List all balance changes for a specific user
cargo run --bin audit user-balances 123456789012345678

# Show a summary of all balance changes
cargo run --bin audit balance-summary

# Show help
cargo run --bin audit help
```

### Audit Tool Output Examples

#### User Commands

```txt
Commands executed by user 123456789012345678:
Timestamp               Command        Guild           Arguments                                 Result    
----------------------------------------------------------------------------------------------------
2025-03-11T18:30:45.123Z give           123456789012345 amount: 50, user: SomeUser#1234          success   
2025-03-11T18:35:12.456Z balance        123456789012345 user: self, global: false                success   
```

#### User Balances

```txt
Balance changes for user 987654321098765432:
Timestamp               Guild           Previous        New             Change    Reason               Initiator      
--------------------------------------------------------------------------------------------------------------
2025-03-11T18:30:45.123Z 123456789012345 100             150             +50       give_command         123456789012345
2025-03-11T19:15:22.789Z 123456789012345 150             149             -1        flip_bet             987654321098765
```

#### Balance Summary

```txt
Balance Change Summary:

Guild: 123456789012345678
User ID                Net Change     
-----------------------------------
987654321098765432     +49            
456789012345678901     +25            
234567890123456789     -10            

Global Summary:
User ID                Net Change     
-----------------------------------
987654321098765432     +49            
456789012345678901     +25            
234567890123456789     -10            
```

## Voting System

The AndyCoin bot includes a democratic voting system that allows server members to vote on resetting all AndyCoins in the server. This feature is designed to add an element of chaos and community engagement.

### How Voting Works

1. Any user can start a vote using `/vote` with the "Start a new vote" option
2. Users can cast their votes using `/vote` with "Vote yes" or "Vote no" options
3. The vote runs for a configurable duration (default: 30 minutes)
4. If the vote passes (default: requires at least 10 votes with 70% majority), all AndyCoins in the server are reset to 0
5. After a vote, there's a cooldown period (default: 24 hours) before another vote can be started

### Vote Configuration

Server administrators can configure the voting system using `/vote_admin config`:

- `cooldown_hours` - Hours between votes (default: 24)
- `duration_minutes` - How long votes last (default: 30)
- `min_votes` - Minimum number of votes required (default: 10)
- `majority_percentage` - Percentage of YES votes required to pass (default: 70)

### Vote Status

Anyone can check the status of an ongoing vote using `/vote_admin status`, which shows:

- Who initiated the vote
- When the vote will end
- Current vote counts and percentages
- Whether the vote would pass or fail with current numbers

## Environment Variables

The logging system respects the following environment variables:

- `RUST_LOG` - Controls the log level (e.g., `info`, `debug`, `trace`)
- `DISCORD_TOKEN` - Your Discord bot token (required)

## Conclusion

AndyCoin Bot is now fully containerized and can be deployed in various environments:

- **Docker/Docker Compose**: For simple, local deployments
- **Kubernetes**: For scalable, orchestrated deployments
- **AWS ECS**: For managed container deployments on AWS
- **Azure App Service/Container Instances**: For managed container deployments on Azure
- **Cloudflare Workers**: For monitoring, health checks, and backups

Each deployment option provides persistent storage for the bot's data, ensuring that user balances and configurations are preserved across restarts and updates. Choose the deployment option that best fits your infrastructure and requirements.

For any issues or questions, please open an issue on the repository.
