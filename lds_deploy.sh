#!/bin/bash
set -e

ALLOW_DEPLOY_ON_MAIN="false"
ALLOW_UNCOMMITED="false"

year=$(date --utc +'%-Y')
month=$(date --utc +'%-m')

new_ver="$year.$month.0"
if git tag | rg "^$year\.$month\." > /dev/null ; then
  new_ver="$year.$month.$(($(git tag | rg -r '$1' "^$year\.$month\.([0-9]*)" | tail -n 1)+1))"
fi

if [[ "$(git status -s)" != "" ]] && [[ "$ALLOW_UNCOMMITED" == "false" ]] ; then
  echo "Uncommitted changes"
  exit 1
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if ! [[ "$BRANCH" != "dev" ]] && [[ "$ALLOW_DEPLOY_ON_MAIN" == "false" ]] ; then
  echo "Not on dev branch"
  exit 1
fi
echo "new version: $new_ver"

echo "Bumping Cargo.toml..."
new_cargo_toml=$(cat Cargo.toml | rg --passthru 'version\s*=\s*"([0-9]*\.[0-9]*\.[0-9]*)"' -r "version=\"$new_ver\"")
echo "$new_cargo_toml" > Cargo.toml

echo "Bumping PKGBUILD..."
new_pkgbuild=$(cat PKGBUILD | rg --passthru 'pkgver\s*=\s*([0-9]*\.[0-9]*\.[0-9]*)' -r "pkgver=$new_ver")
echo "$new_pkgbuild" > PKGBUILD


echo "Checking into version control..."
git add .
git commit -m "bump: to v$new_ver"

git tag -a $new_ver -m "v$new_ver"

echo "Pushing to remote"
git push
git push --tags

if [[ "$BRANCH" != "main" ]] ; then
  echo "Merging into main branch..."
  git checkout main
  git merge dev
  git push
  git push --tags
  git checkout dev
else
  echo "No need to merge into main branch"
fi

echo "done"
