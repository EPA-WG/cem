#!/usr/bin/env bash

echo "🚀 Starting release preparation..."

# Step 1: Run Nx release (bump versions, create changelog)
echo "📦 Running Nx release..."
set +e  # Temporarily disable exit on error
yarn nx release --skip-publish
RELEASE_EXIT_CODE=$?
set -e  # Re-enable exit on error

if [ $RELEASE_EXIT_CODE -ne 0 ]; then
    # If nx release fails (e.g., tag already exists), check if we can continue
    NEW_VERSION=$(node -p "require('./package.json').version")
    if git rev-parse "$NEW_VERSION" >/dev/null 2>&1; then
        echo "⚠️  Release already created, continuing with workspace protocol replacement..."
    else
        echo "❌ Release failed with unexpected error (exit code: $RELEASE_EXIT_CODE)"
        exit $RELEASE_EXIT_CODE
    fi
else
    echo "✅ Release created successfully"
fi

# Step 2: Replace workspace:* protocol with actual versions
echo "🔄 Replacing workspace protocol with actual versions..."
set -e  # Enable exit on error for remaining steps
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
