#!/bin/bash
set -e
PROJECT_NAME=$(grep "^name =" Cargo.toml | head -n1 | cut -d"\"" -f2)
if [ -z "$1" ]; then
    VERSION=$(grep "^version =" Cargo.toml | head -n1 | cut -d"\"" -f2)
else
    VERSION=$1
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
fi
if [ -f "package.json" ]; then sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json; fi
if [ -f "RELEASE_NOTES.md" ]; then
    sed -i "s/Welcome to .* CLI v[0-9.]*/Welcome to $PROJECT_NAME CLI v$VERSION/" RELEASE_NOTES.md
    sed -i "s/What's New in v[0-9.]*/What's New in v$VERSION/" RELEASE_NOTES.md
fi
echo "✅ $PROJECT_NAME updated to v$VERSION"
if command -v gh &> /dev/null; then
    echo "Updating GitHub release v$VERSION..."
    gh release edit "v$VERSION" --notes-file RELEASE_NOTES.md || \
    gh release create "v$VERSION" --title "v$VERSION" --notes-file RELEASE_NOTES.md
fi

