# setup
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.4/install.sh | bash
\. "$HOME/.nvm/nvm.sh"
nvm install 24
npm install -g corepack
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
sudo apt-get install -y build-essential
sudo apt-get install -y libnss3 libnspr4 libasound2t64
PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64 yarn playwright install # use var only for Ubuntu

npm install -g @openai/codex

```
