#!/usr/bin/env bash
set -euo pipefail

# Only the publishing job receives a write token. Build jobs never do.
: "${GH_REPO:?}" "${GITHUB_SHA:?}" "${GITHUB_REF:?}" "${DEFAULT_BRANCH:?}"
python3 packaging/release.py notes "$GITHUB_REF" release-notes.md
python3 packaging/release.py collect artifacts dist "$GITHUB_SHA"
printf '\nBuilt from %s.\n' "$GITHUB_SHA" >> release-notes.md

if [[ $GITHUB_REF == refs/tags/v* ]]; then
    tag=${GITHUB_REF#refs/tags/}
    title="Tenchu: Shinobi Gaisen PC $tag"
    prerelease=false
    [[ $tag != *-* ]] || prerelease=true
    latest=$([[ $prerelease == true ]] && echo false || echo legacy)
else
    if [[ $GITHUB_REF != "refs/heads/$DEFAULT_BRANCH" ]]; then
        echo 'Only the default branch publishes HEAD.'
        exit 0
    fi
    current=$(gh api "repos/$GH_REPO/git/ref/heads/$DEFAULT_BRANCH" --jq .object.sha)
    if [[ $current != "$GITHUB_SHA" ]]; then
        echo 'A newer default-branch commit exists; keep the current HEAD download.'
        exit 0
    fi
    tag='head'
    title='HEAD (development)'
    prerelease=true
    latest=false
fi

# Query the release list so authentication/network failures cannot look like 404s.
existing=$(gh api --paginate "repos/$GH_REPO/releases?per_page=100" \
    --jq ".[] | select(.tag_name == \"$tag\") | [.id, .draft, .immutable] | @tsv")
if [[ -n $existing ]]; then
    read -r id draft immutable <<< "$existing"
    if [[ $immutable == true ]]; then
        echo "Release $tag is immutable; it cannot be replaced." >&2
        exit 1
    fi
    if [[ $tag != head && $draft != true ]]; then
        echo "Release $tag is already published. Publish fixes under a new version." >&2
        exit 1
    fi
    # Make HEAD unavailable while replacing it, so downloads cannot mix builds.
    gh release edit "$tag" --draft --latest=false
    assets=$(gh api --paginate "repos/$GH_REPO/releases/$id/assets" --jq '.[].id')
    while IFS= read -r asset; do
        [[ -z $asset ]] || gh api --method DELETE "repos/$GH_REPO/releases/assets/$asset"
    done <<< "$assets"
fi

if [[ $tag == head ]]; then
    refs=$(gh api "repos/$GH_REPO/git/matching-refs/tags/head" --jq '.[] | select(.ref == "refs/tags/head") | .ref')
    if [[ -n $refs ]]; then
        gh api --method PATCH "repos/$GH_REPO/git/refs/tags/head" -f sha="$GITHUB_SHA" -F force=true
    else
        gh api --method POST "repos/$GH_REPO/git/refs" -f ref=refs/tags/head -f sha="$GITHUB_SHA"
    fi
fi

if [[ -z $existing ]]; then
    gh release create "$tag" --verify-tag --draft --title "$title" \
        --notes-file release-notes.md --prerelease="$prerelease" --latest=false
else
    gh release edit "$tag" --title "$title" --notes-file release-notes.md --prerelease="$prerelease"
fi
gh release upload "$tag" dist/*
id=$(gh release view "$tag" --json databaseId --jq .databaseId)
gh api --method PATCH "repos/$GH_REPO/releases/$id" -F draft=false -f make_latest="$latest"
