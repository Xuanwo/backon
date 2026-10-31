#!/usr/bin/env bash
set -euo pipefail

tools_dir=$1
node_version=v24.22.2
archive="node-$node_version-linux-x64.tar.xz"
mkdir -p "$tools_dir"
cd "$tools_dir"
curl -fsSL "https://nodejs.org/dist/$node_version/$archive" --output "$archive"
curl -fsSL "https://nodejs.org/dist/$node_version/SHASUMS256.txt" --output SHASUMS256.txt
sha256sum --check --ignore-missing SHASUMS256.txt
tar -xf "$archive"
"./node-$node_version-linux-x64/bin/node" --version
