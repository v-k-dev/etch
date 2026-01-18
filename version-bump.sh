#!/bin/bash
# version-bump.sh - Helper script for creating version releases

set -e

CURRENT_VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//' || echo "0.1.0")
echo "Current version: $CURRENT_VERSION"

# Parse version - handle empty case
if [ -z "$CURRENT_VERSION" ]; then
    CURRENT_VERSION="0.1.0"
fi

IFS='.' read -ra VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR="${VERSION_PARTS[0]:-0}"
MINOR="${VERSION_PARTS[1]:-1}"
PATCH="${VERSION_PARTS[2]:-0}"

# Analyze recent commits since last tag
COMMITS=$(git log $(git describe --tags --abbrev=0 2>/dev/null || echo "")..HEAD --oneline --format=%s 2>/dev/null || git log --oneline --format=%s)

BUMP_TYPE="patch"

while IFS= read -r commit; do
    lower=$(echo "$commit" | tr '[:upper:]' '[:lower:]')
    
    if [[ $lower =~ ^(breaking|major): ]] || [[ $lower =~ !: ]]; then
        BUMP_TYPE="major"
        break
    elif [[ $lower =~ ^(feat|feature|add): ]]; then
        if [ "$BUMP_TYPE" != "major" ]; then
            BUMP_TYPE="minor"
        fi
    fi
done <<< "$COMMITS"

# Calculate new version
case $BUMP_TYPE in
    major)
        NEW_VERSION="$((MAJOR + 1)).0.0"
        ;;
    minor)
        NEW_VERSION="$MAJOR.$((MINOR + 1)).0"
        ;;
    patch)
        NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
        ;;
esac

echo "Detected bump type: $BUMP_TYPE"
echo "New version: $NEW_VERSION"
echo ""
echo "Recent commits:"
echo "$COMMITS" | head -5
echo ""

read -p "Create tag v$NEW_VERSION and push? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
    echo "Tag created: v$NEW_VERSION"
    
    read -p "Push tag to origin? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git push origin "v$NEW_VERSION"
        echo "Tag pushed to origin"
        echo ""
        echo "Next steps:"
        echo "1. Build release binary: cargo build --release"
        echo "2. Create GitHub release at: https://github.com/v-k-dev/etch/releases/new"
        echo "3. Upload target/release/etch as 'etch-x86_64'"
    fi
else
    echo "Cancelled."
fi
