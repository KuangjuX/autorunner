# autorunner

CLI tool to sync running data from COROS watches and display on your personal website.

## Usage

### Local

```bash
# Copy and fill in your credentials
cp .env.example .env

# Sync running data from COROS
cargo run -- sync --output running_data.json

# View summary from existing data
cargo run -- summary --input running_data.json

# Generate website JS file
python scripts/generate_running_js.py running_data.json /path/to/website/js/data/running.js
```

### GitHub Actions

The workflow runs daily and:
1. Builds the Rust binary
2. Fetches running data from COROS
3. Generates `running.js` for the website
4. Commits and pushes to the website repo

Required secrets: `COROS_ACCOUNT`, `COROS_PASSWORD`, `PERSONAL_TOKEN`