#!/usr/bin/env bash

set -e  # Exit on error

echo "🚀 Starting release preparation..."

# Step 1: Run Nx release (bump versions, create changelog)
echo "📦 Running Nx release..."
yarn nx release --skip-publish

# Step 2: Replace workspace:* protocol with actual versions
echo "🔄 Replacing workspace protocol with actual versions..."
node tools/scripts/replace-workspace-protocol.cjs

# Step 3: Update yarn.lock
echo "🔒 Updating yarn.lock..."
yarn install

# Step 4: Stage changes
echo "📝 Staging changes..."
git add packages/*/package.json yarn.lock

# Step 5: Amend the release commit
echo "✏️  Amending release commit..."
git commit --amend --no-edit

# Step 6: Push commits and tags
echo "⬆️  Pushing to remote..."
git push --force-with-lease
git push --tags

echo "✅ Release preparation complete!"
echo "🎉 Ready to publish via CI/CD"
