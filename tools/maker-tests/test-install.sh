#!/usr/bin/env bash
# Harness for install_copy (the make-usbroot.sh install-copy engine).
# Splices the install-offer block out of the real maker and runs the four
# data-safety scenarios against plain directories: fresh install, identical
# rerun, foreign-file refusal (nothing written), nvuu-stick.json upgrade
# (overwrite ours, preserve theirs).
# Usage: bash tools/maker-tests/test-install.sh [usbroot-fixture]
# Build the fixture first, e.g.:
#   bash release/make-usbroot.sh --family tu10x --no-install-prompt --out .relcheck/usbroot-fixture
set -uo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$REPO/release/make-usbroot.sh"
OUT_ROOT="${1:-$REPO/.relcheck/usbroot-fixture}"
TARGET="$REPO/.relcheck/copy-target-sh"
[ -f "$OUT_ROOT/EFI/BOOT/BOOTX64.EFI" ] || { echo "fixture missing: $OUT_ROOT (build it with make-usbroot.sh --no-install-prompt first)"; exit 1; }

awk '/^# ---------- install-target probe/,0' "$SRC" > /tmp/install-block.sh
HOST_WIN=0
INSTALL_TO=""
NO_INSTALL_PROMPT=1
OUT="$OUT_ROOT"
# stubs for helpers defined earlier in the real script (line 87-92)
log() { printf '%s\n' "$*"; }
logerr() { printf '%s\n' "$*" >&2; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }
sha256_of() { sha256sum "$1" | awk '{print $1}'; }
# shellcheck disable=SC1090
source /tmp/install-block.sh   # defines the functions, runs install_offer (prompt off)

rm -rf "$TARGET"
# 1. fresh install
mkdir -p "$TARGET"
install_copy "$OUT_ROOT" "$TARGET" || { echo "TEST1 fresh: FAIL"; exit 1; }
[ -f "$TARGET/EFI/BOOT/BOOTX64.EFI" ] || { echo "TEST1 FAIL: EFI/BOOT missing"; exit 1; }
echo "TEST1 fresh: ok"

# 2. identical rerun
install_copy "$OUT_ROOT" "$TARGET" || { echo "TEST2 identical: FAIL"; exit 1; }
echo "TEST2 identical rerun: ok"

# 3. foreign differing file -> refuse
rm -rf "$TARGET" && mkdir -p "$TARGET"
printf 'user data' > "$TARGET/some-user-file.txt"
printf 'foreign bytes' > "$TARGET/booter_load_tu10x.bin"
before=$(sha256sum "$TARGET/some-user-file.txt" | awk '{print $1}')
if install_copy "$OUT_ROOT" "$TARGET"; then echo "TEST3 refuse: FAIL (should refuse)"; exit 1; fi
after=$(sha256sum "$TARGET/some-user-file.txt" | awk '{print $1}')
[ "$before" = "$after" ] || { echo "TEST3 FAIL: user file modified"; exit 1; }
echo "TEST3 conflict refuse: ok (nothing written)"

# 4. our marker -> upgrade
printf '{\n  "project": "nv-uefi-unlocker"\n}\n' > "$TARGET/nvuu-stick.json"
install_copy "$OUT_ROOT" "$TARGET" || { echo "TEST4 upgrade: FAIL"; exit 1; }
[ "$(sha256_of "$TARGET/booter_load_tu10x.bin")" = "$(sha256_of "$OUT_ROOT/booter_load_tu10x.bin")" ] || { echo "TEST4 FAIL: not restored"; exit 1; }
[ -f "$TARGET/some-user-file.txt" ] || { echo "TEST4 FAIL: user file deleted"; exit 1; }
echo "TEST4 marker upgrade: ok (user file preserved)"

echo "ALL SH INSTALL HARNESS TESTS PASSED"
