#!/bin/sh
# web/editor.js from entry.js: CodeMirror 6, its languages, marked and DOMPurify (the versions in package.json), bundled
# with esbuild into one minified file the page loads as it is (window.Editor), their licences kept at its end.
# Installs into a temporary folder, without install scripts; nothing is left in the checkout but editor.js.
#   web/editor/build.sh
set -e
here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cp "$here/package.json" "$tmp/"
(cd "$tmp" && npm install --ignore-scripts --no-audit --no-fund --silent)
cp "$here/entry.js" "$tmp/entry.js"
"$tmp/node_modules/.bin/esbuild" "$tmp/entry.js" --bundle --minify --format=iife --legal-comments=eof --target=es2020 --outfile="$here/../editor.js" \
  --banner:js="/* burrow's editor: CodeMirror 6, Lezer (MIT), marked (MIT), DOMPurify (Apache-2.0 / MPL-2.0); their licences in web/editor/LICENSES.txt. Built by web/editor/build.sh */"
node "$here/licenses.mjs" "$tmp/node_modules" > "$here/LICENSES.txt"
ls -l "$here/../editor.js" "$here/LICENSES.txt"
