#!/bin/bash
# Update version references in README.md and docs before committing a version bump.

NEW_VERSION=$CZ_PRE_NEW_VERSION

# Cross-platform sed in-place edit (macOS vs Linux)
sedi() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

for file in README.md docs/integrations/prek.md docs/integrations/github-action.md; do
    sedi -E "s/rev: v[0-9]+\.[0-9]+\.[0-9]+/rev: v$NEW_VERSION/" "$file"
    sedi -E "s/uses: feliblo\/odcslint@v[0-9]+\.[0-9]+\.[0-9]+/uses: feliblo\/odcslint@v$NEW_VERSION/" "$file"
done
