#!/usr/bin/env bash
# Prints the BtbN FFmpeg build that the Dockerfile pins, as KEY=value lines
# (for $GITHUB_ENV), so the desktop installer bundles the same build:
#   FFMPEG_TAG      BtbN release tag, e.g. autobuild-2026-08-31-13-27
#   FFMPEG_VERSION  FFmpeg version, e.g. n8.1.2-50-g1a748fe2cd
#   FFMPEG_BRANCH   FFmpeg release branch of the build, e.g. 8.1
#   FFMPEG_COMMIT   abbreviated FFmpeg commit, e.g. 1a748fe2cd
set -euo pipefail

dockerfile="${1:-Dockerfile}"
url=$(grep -o "https://github.com/BtbN/FFmpeg-Builds/releases/download/[^']*" "$dockerfile" | head -n 1 || true)
asset=$(grep -o "linux64-gpl-[0-9.]*\.tar\.xz" "$dockerfile" | head -n 1 || true)
url_re='/download/(autobuild-[^/]+)/ffmpeg-(.+-g([0-9a-f]+))-$'
asset_re='^linux64-gpl-([0-9.]+)\.tar\.xz$'
if [[ ! $url =~ $url_re ]]; then
  echo "no BtbN FFmpeg URL found in $dockerfile" >&2
  exit 1
fi
echo "FFMPEG_TAG=${BASH_REMATCH[1]}"
echo "FFMPEG_VERSION=${BASH_REMATCH[2]}"
echo "FFMPEG_COMMIT=${BASH_REMATCH[3]}"
if [[ ! $asset =~ $asset_re ]]; then
  echo "no linux64 GPL asset found in $dockerfile" >&2
  exit 1
fi
echo "FFMPEG_BRANCH=${BASH_REMATCH[1]}"
