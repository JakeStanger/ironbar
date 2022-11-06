#!/bin/sh
set -eu

TEMP_REPO_DIR="wiki_action_$GITHUB_REPOSITORY$GITHUB_SHA"
TEMP_WIKI_DIR="temp_wiki_$GITHUB_SHA"

WIKI_DIR='docs'

if [ -z "$GH_TOKEN" ]; then
    echo "Token is not specified"
    exit 1
fi

#Clone repo
echo "Cloning repo https://github.com/$GITHUB_REPOSITORY"
git clone "https://$GITHUB_ACTOR:$GH_TOKEN@github.com/$GITHUB_REPOSITORY" "$TEMP_REPO_DIR"

#Clone wiki repo
echo "Cloning wiki repo https://github.com/$GITHUB_REPOSITORY.wiki.git"
cd "$TEMP_REPO_DIR"
git clone "https://$GITHUB_ACTOR:$GH_TOKEN@github.com/$GITHUB_REPOSITORY.wiki.git" "$TEMP_WIKI_DIR"

#Get commit details
author='Jake Stanger'
email='mail@jstanger.dev'
message='action: sync wiki'

echo "Copying edited wiki"
cp -R "$TEMP_WIKI_DIR/.git" "$WIKI_DIR/"

echo "Checking if wiki has changes"
cd "$WIKI_DIR"
git config --local user.email "$email"
git config --local user.name "$author"
git add .

if git diff-index --quiet HEAD; then
  echo "Nothing changed"
  exit 0
fi

echo "Pushing changes to wiki"
git commit -m "$message" && git push "https://$GITHUB_ACTOR:$GH_TOKEN@github.com/$GITHUB_REPOSITORY.wiki.git"