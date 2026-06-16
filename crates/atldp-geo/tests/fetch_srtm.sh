#!/usr/bin/env bash
# Download SRTM tile S23W043 from the AWS elevation-tiles-prod bucket (public,
# no auth required).  Tile covers lat -23 to -22 / lon -43 to -42 (Rio de
# Janeiro state, Brazil).
#
# Run from the repo root or from this script's directory; the .hgt file is
# placed in the same `data/` directory as this script.
#
# Usage: bash crates/atldp-geo/tests/fetch_srtm.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$SCRIPT_DIR/data"
URL="https://s3.amazonaws.com/elevation-tiles-prod/skadi/S23/S23W043.hgt.gz"
GZ_FILE="$DATA_DIR/S23W043.hgt.gz"
HGT_FILE="$DATA_DIR/S23W043.hgt"

mkdir -p "$DATA_DIR"

if [[ -f "$HGT_FILE" ]]; then
    SIZE=$(stat -c%s "$HGT_FILE" 2>/dev/null || stat -f%z "$HGT_FILE")
    echo "Already downloaded: $HGT_FILE ($SIZE bytes)"
    exit 0
fi

echo "Downloading $URL ..."
curl -fL --progress-bar -o "$GZ_FILE" "$URL"
echo "Decompressing ..."
gunzip -f "$GZ_FILE"
SIZE=$(stat -c%s "$HGT_FILE" 2>/dev/null || stat -f%z "$HGT_FILE")
echo "Done: $HGT_FILE ($SIZE bytes)"

# Sanity check: SRTM3 tile = 1201 × 1201 × 2 = 2884802 bytes.
# SRTM1 tile  = 3601 × 3601 × 2 = 25934402 bytes.
if [[ "$SIZE" != "2884802" && "$SIZE" != "25934402" ]]; then
    echo "WARNING: unexpected file size $SIZE (expected 2884802 or 25934402)" >&2
fi
