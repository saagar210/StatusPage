#!/usr/bin/env bash
set -euo pipefail

# codex-os-managed
max_bytes="${ASSET_MAX_BYTES:-350000}"
asset_root="apps/web/public"
if [[ ! -d "$asset_root" ]]; then
  echo "No web public directory found; skipping asset check."
  exit 0
fi

fail=0
while IFS= read -r file; do
  size=$(wc -c < "$file")
  if (( size > max_bytes )); then
    echo "Asset too large (>${max_bytes} bytes): $file"
    fail=1
  fi
done < <(find "$asset_root" -type f \( -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" -o -name "*.webp" -o -name "*.avif" \))

exit $fail
