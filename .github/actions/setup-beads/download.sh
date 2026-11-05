#!/usr/bin/env bash
# Injectable command seams keep retry tests deterministic without network or waits.
if ! declare -F beads_download_once >/dev/null; then
  beads_download_once() { curl --fail --location --silent --show-error --output "$2" "$1"; }
fi
if ! declare -F beads_retry_sleep >/dev/null; then
  beads_retry_sleep() { sleep "$1"; }
fi

download_beads_release() {
  local url="$1" destination="$2" attempt status delay
  for attempt in 1 2 3 4 5; do
    if beads_download_once "$url" "$destination"; then
      return 0
    else
      status=$?
    fi
    if (( attempt == 5 )); then
      echo "BEADS.RELEASE_DOWNLOAD_FAILED: failed to download ${url} after ${attempt} attempts (curl exit ${status})" >&2
      return "$status"
    fi
    delay=$((1 << (attempt - 1)))
    echo "Beads release download failed (curl exit ${status}); retrying after ${delay}s" >&2
    beads_retry_sleep "$delay"
  done
}
