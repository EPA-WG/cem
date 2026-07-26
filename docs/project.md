# setup
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.4/install.sh | bash
\. "$HOME/.nvm/nvm.sh"
nvm install 24
npm install -g corepack
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
sudo apt-get install -y build-essential
sudo apt-get install -y poppler-utils # pdftocairo for README SVG previews
sudo apt-get install -y libnss3 libnspr4 libasound2t64 # Playwright/headless Chromium for e2e and README previews
PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64 yarn playwright install # headless browser for e2e and README PDF previews; use var only for Ubuntu

npm install -g @openai/codex

```
