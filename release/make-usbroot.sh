#!/usr/bin/env bash
# make-usbroot.sh — bash twin of make-usbroot.ps1 (Linux / WSL / Git Bash).
#
# Regenerates the unlock USB stick root from pinned public inputs. NVIDIA
# firmware blobs are NOT distributed with this project; this script downloads
# the pinned public packages, verifies their SHA-256, and regenerates every
# file the EFI application needs for the chosen GPU family.
#
# Usage:
#   ./make-usbroot.sh                          (ad102)
#   ./make-usbroot.sh --family tu10x --out /media/usb
#
# Options (1:1 with the PowerShell twin's parameters):
#   --family F        ad102 | ga102 | tu10x (aliases rtx40/40, rtx30/30, rtx20/20;
#                     default ad102)
#   --out DIR         output root (default ./usbroot)
#   --work DIR        download/verify work dir (default ./inputs)
#   --assets PATH     assets pipeline binary. Resolution order: this flag /
#                     $ASSETS_BIN, then 'assets-x86_64-linux' or 'assets' next
#                     to this script (release zip), then an automatic
#                     `cargo build -p assets` in the enclosing repo checkout.
#   --boot-efi PATH   BOOTX64.EFI to place on the stick (default: next to script)
#   --proven-blob P   explicit 128 KiB cmp50hx GspRmBoot archive (tu10x only)
#   --skip-gsp-check  build for another machine: skip the host Turing GSP check
#   -h | --help       this text
#
# Family notes (same as make-usbroot.ps1):
#   ad102  RTX 40 series (Ada)          - no host prerequisites
#   ga102  RTX 30 series (Ampere)       - kmod-verified upstream, no silicon
#                                         evidence for this EFI path yet
#   tu10x  RTX 20 / 16 series (Turing)  - REQUIRES GSP-RM firmware mode on the
#                     TARGET machine; the legacy path TDR-loops on the
#                     unlocked card. On Linux the proprietary driver 545+
#                     runs GSP-RM by default on Turing and newer (no
#                     EnableGpuFirmware-style switch exists or is needed).
#                     The host gate therefore refuses to build when
#                     nvidia-smi reports the GSP firmware N/A - driver too
#                     old, or NVreg_EnableGpuFirmware=0 forced in a modprobe
#                     config. Building for a DIFFERENT machine? Pass
#                     --skip-gsp-check.
#
# RTD3 notice (issue #9): when the HOST runs with runtime D3 enabled
# (DynamicPowerManagement 0x02/0x03), the script prints a loud warning -
# RTD3 parks the GPU in D3cold when idle and D3cold ERASES the volatile
# unlock after handoff (green loader.log, no gain in the OS). See the
# README section "Linux note: runtime D3".
#
# Requires: bash 4+, GNU stat/sha256sum/date, curl or wget, internet access
# to the pinned URLs. macOS is not supported (GNU stat/date dependency).

set -euo pipefail

FAMILY="ad102"
OUT="./usbroot"
WORK="./inputs"
ASSETS_BIN="${ASSETS_BIN:-}"
BOOT_EFI=""
PROVEN_BLOB=""
SKIP_GSP_CHECK=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# Windows-hosted bash (Git Bash/MSYS) can run .exe pipelines, not Linux ELF;
# the release ships assets-x86_64-linux for real Linux and
# assets-x86_64-windows.exe for Windows hosts.
HOST_WIN=0
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) HOST_WIN=1 ;; esac

usage() {
    awk 'NR > 1 { if ($0 !~ /^#/) exit; sub(/^# ?/, ""); print }' "$0"
    exit 0
}

log() { printf '%s\n' "$*"; }
logerr() { printf '%s\n' "$*" >&2; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }
sha256_of() { sha256sum "$1" | awk '{print $1}'; }
size_of() { stat -c %s "$1"; }

fetch() { # fetch <url> <dest>
    if have curl; then
        curl -fsSL --retry 3 -o "$2" "$1" || die "download failed: $1"
    elif have wget; then
        wget -q -O "$2" "$1" || die "download failed: $1"
    else
        die "need curl or wget to download $1"
    fi
}

prompt_continue() {
    local a=""
    if [ -t 0 ]; then
        read -r -p "Continue anyway? (y/N) " a || a="n"
    else
        a="n"
    fi
    [ "${a,,}" = "y" ] || [ "${a,,}" = "yes" ]
}

# ---------- pinned public inputs (see AGENTS.md / assets/src/manifest.rs) ----------
SRC1_NAME="NVIDIA-kernel-module-source-610.43.03.tar.xz"
SRC1_URL="https://download.nvidia.com/XFree86/NVIDIA-kernel-module-source/NVIDIA-kernel-module-source-610.43.03.tar.xz"
SRC1_SHA="7e118923c7a23edc36114d63273a46e3e04e9af98695a42203e7ac2dfe9fc1dc"
SRC2_NAME="nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
SRC2_URL="https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
SRC2_SHA="e17e7c92eeffa178b0fe1f40eb17a47095f8fc3ce4a77bf115cf5bd5a4df86e9"

# Provenance-pinned Turing BOOT_DIRECT bootloader. 0x6000 is not an arbitrary
# truncation: it is GSP_RM_BOOT_SIZE (cmp50hx kernel source), the exact length
# the booter consumes. The slice (de676bae...) fired the successful TU106 run
# (2026-10-27, attempt 13: "gsp_boot_tu10x.bin loaded 24576/24576 bytes").
# The GSP ELF's first 0x6000 bytes (c76c43b1...) is NOT the same image and
# must not ship.
PROVEN_NAME="gsp_rm_boot_dbg.bin"
PROVEN_SRC_SHA="fa43239bcee7b97ca62f007cc68487560a39e19f74f3dde7486db3f98df8e471"
PROVEN_URL="https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
PROVEN_SLICE_LEN=24576   # 0x6000
PROVEN_SLICE_SHA="de676bae28a480011d3d012db14bef539324e62a841a9627863c689bea168af3"

# ---------- arg parsing ----------
while [ $# -gt 0 ]; do
    case "$1" in
        --family)          FAMILY="${2:?missing value for $1}"; shift 2 ;;
        --family=*)        FAMILY="${1#*=}"; shift ;;
        --out)             OUT="${2:?missing value for $1}"; shift 2 ;;
        --out=*)           OUT="${1#*=}"; shift ;;
        --work)            WORK="${2:?missing value for $1}"; shift 2 ;;
        --work=*)          WORK="${1#*=}"; shift ;;
        --assets)          ASSETS_BIN="${2:?missing value for $1}"; shift 2 ;;
        --assets=*)        ASSETS_BIN="${1#*=}"; shift ;;
        --boot-efi)        BOOT_EFI="${2:?missing value for $1}"; shift 2 ;;
        --boot-efi=*)      BOOT_EFI="${1#*=}"; shift ;;
        --proven-blob)     PROVEN_BLOB="${2:?missing value for $1}"; shift 2 ;;
        --proven-blob=*)   PROVEN_BLOB="${1#*=}"; shift ;;
        --skip-gsp-check)  SKIP_GSP_CHECK=1; shift ;;
        -h|--help)         usage ;;
        *) die "unknown option: $1 (see --help)" ;;
    esac
done

FAMILY="${FAMILY,,}"
case "$FAMILY" in
    ad102|rtx40|40) FAMILY="ad102" ;;
    ga102|rtx30|30) FAMILY="ga102" ;;
    tu10x|rtx20|20) FAMILY="tu10x" ;;
    *) die "unknown family: $FAMILY (ad102 | ga102 | tu10x)" ;;
esac

# ---------- tools ----------
have stat || die "stat not found"
size_of /dev/null >/dev/null 2>&1 || die "GNU stat required (Linux/WSL/Git Bash; macOS not supported)"
have sha256sum || die "sha256sum not found"

resolve_assets() {
    if [ -n "$ASSETS_BIN" ] && [ -x "$ASSETS_BIN" ]; then return 0; fi
    local cand
    local cands
    if [ "$HOST_WIN" -eq 1 ]; then
        cands=("$SCRIPT_DIR/assets-x86_64-windows.exe" "$SCRIPT_DIR/assets.exe")
    else
        # The release zip carries no Unix exec bits: heal +x on our own copies.
        cands=("$SCRIPT_DIR/assets-x86_64-linux" "$SCRIPT_DIR/assets")
    fi
    for cand in "${cands[@]}"; do
        if [ -f "$cand" ]; then
            [ -x "$cand" ] || chmod +x "$cand" 2>/dev/null || true
            if [ -x "$cand" ]; then ASSETS_BIN="$cand"; return 0; fi
        fi
    done
    # Repo checkout? (the script lives in release/, repo root one level up)
    if [ -f "$SCRIPT_DIR/../Cargo.toml" ] && [ -f "$SCRIPT_DIR/../assets/Cargo.toml" ]; then
        if have cargo; then
            log "[build] building host assets pipeline (cargo build -p assets)"
            (cd "$SCRIPT_DIR/.." && cargo build --release --quiet -p assets) \
                || die "cargo build -p assets failed"
            ASSETS_BIN="$SCRIPT_DIR/../target/release/assets"
            if [ "$HOST_WIN" -eq 1 ]; then ASSETS_BIN="$SCRIPT_DIR/../target/release/assets.exe"; fi
            [ -x "$ASSETS_BIN" ] || die "assets binary missing after build: $ASSETS_BIN"
            return 0
        fi
        die "assets pipeline not found and cargo is missing.
Install Rust (https://rustup.rs) so the pipeline can be built from the
repo sources, or point --assets at a prebuilt 'assets' binary."
    fi
    die "assets pipeline not found.
Provide it via --assets or \$ASSETS_BIN, place a Linux build named
'assets-x86_64-linux' next to this script (the release zip ships one),
or run this script from a repo checkout so it can cargo-build it."
}

assets_run() {
    log "  [assets] $*"
    "$ASSETS_BIN" "$@" || die "assets $1 failed"
}

# ---------- RTD3 host notice (issue #9) ----------
rtd3_notice() {
    [ -r /proc/driver/nvidia/params ] || return 0
    local v
    v="$(grep -E '^DynamicPowerManagement:' /proc/driver/nvidia/params | head -n1 | awk '{print $2}' || true)"
    case "${v,,}" in
        0x02|0x2|2|0x03|0x3|3)
            log "[rtd3] WARNING: this host runs the NVIDIA driver with runtime D3 enabled"
            log "[rtd3]        (DynamicPowerManagement=$v). RTD3 parks the GPU in D3cold when"
            log "[rtd3]        idle, and D3cold ERASES the volatile unlock after handoff"
            log "[rtd3]        (issue #9): green loader.log, no gain in the OS."
            log "[rtd3]        Fix, then reboot:"
            log "[rtd3]          echo 'options nvidia NVreg_DynamicPowerManagement=0x00' | \\"
            log "[rtd3]            sudo tee /etc/modprobe.d/nvidia-runtimepm.conf"
            log "[rtd3]          sudo update-initramfs -u   # Fedora/RHEL: sudo dracut -f"
            log "[rtd3]        (reuse the distro file's basename so it shadows it; verify with:"
            log "[rtd3]         modprobe -c | grep -i DynamicPowerManagement)"
            ;;
    esac
}

# ---------- Turing GSP-mode gate (refuse without GSP firmware mode) ----------
GSP_GATE_STATUS="n/a"
ADAPTER_LINES=""

gsp_state() { # echoes: active | inactive | unknown | failed  (nvidia-smi must exist)
    local q any_field=0 any_inactive=0 line val
    q="$(nvidia-smi -q 2>/dev/null)" || { echo "failed"; return; }
    while IFS= read -r line; do
        any_field=1
        val="${line#*:}"; val="${val//[[:space:]]/}"
        # FAIL CLOSED. This aggregate cannot tell WHICH adapter the field
        # belongs to, so a single N/A has to count against the build: a Turing
        # card left on the legacy init path TDR-loops on the unlocked GPU, and
        # the old any_active rule let a 4090 + 2070 box pass on the 4090's GSP
        # state. Building for another machine? --skip-gsp-check.
        case "${val,,}" in n/a|na|"") any_inactive=1 ;; esac
    done < <(printf '%s\n' "$q" | grep 'GSP Firmware Version' || true)
    if [ "$any_field" -eq 0 ]; then echo "unknown"; return; fi
    [ "$any_inactive" -eq 1 ] && echo "inactive" || echo "active"
}

check_turing_gate() {
    GSP_GATE_STATUS="skipped"
    if [ "$SKIP_GSP_CHECK" -eq 1 ]; then
        log "[gate] Turing GSP check skipped (--skip-gsp-check)"
        return 0
    fi
    log "[gate] Turing family selected - checking host GSP firmware mode"

    if have nvidia-smi; then
        local inv devid gname d dv is_turing=0 _idx
        inv="$(nvidia-smi --query-gpu=index,pci.device_id,name --format=csv,noheader 2>/dev/null || true)"
        ADAPTER_LINES="$(printf '%s\n' "$inv" | sed 's/^/  /')"
        while IFS=, read -r _idx devid gname; do
            [ -n "$devid" ] || continue
            d="${devid,,}"; d="${d// /}"; d="${d#*0x}"; d="${d:0:4}"
            dv=0
            if [ "${#d}" -eq 4 ]; then dv=$((16#$d)) || dv=0; fi
            is_turing=0
            # Same ranges as the PowerShell twin's Test-TuringDevId and
            # tools/windows/enable-turing-unlock.ps1:
            # 0x1E02..0x1FFF (TU102/TU104/TU106/TU116, incl. 16-series) and
            # 0x2182..0x21D1 (mobile TU117 and the 21xx variants - the upper
            # bound used to stop at 0x2192 here, so 0x2193..0x21D1 cards were
            # "no Turing adapter" on Linux and "refused" on Windows).
            if (( dv >= 0x1E02 && dv <= 0x1FFF )) || (( dv >= 0x2182 && dv <= 0x21D1 )); then
                is_turing=1
                log "  Turing adapter ${_idx// /}: ${gname:-?} (0x$d) on this host"
                break
            fi
        done <<< "$inv"

        if [ "$is_turing" -eq 0 ]; then
            log "[gate] No NVIDIA Turing adapter found on THIS machine."
            log "       If the unlock target is a different machine, that is fine."
            prompt_continue || die "Aborted by user."
            GSP_GATE_STATUS="no-turing"
            log "[gate] skipped (no Turing adapter on this host)"
            return 0
        fi

        # Runtime corroboration: a module parameter only proves the switch is
        # SET, not that the current driver session actually loaded GSP firmware
        # (a stale pre-restart session keeps the legacy path). On Linux the
        # nvidia-smi runtime field IS the primary check.
        local state
        state="$(gsp_state)"
        case "$state" in
            active)
                log "[ok] GSP firmware running (nvidia-smi reports a firmware version)"
                ;;
            failed)
                die "nvidia-smi -q failed (driver present but not reporting?).
If this machine is not the unlock target, re-run with --skip-gsp-check."
                ;;
            unknown)
                die "nvidia-smi has no 'GSP Firmware Version' field: the driver is too
old to report (or to support) GSP-RM. Turing unlock requires GSP firmware
mode; update the driver (545+ runs GSP by default on Turing+), RESTART and
re-run. Building for another machine? Use --skip-gsp-check."
                ;;
            inactive)
                die "Turing adapter(s) on this host are NOT in GSP firmware mode
(nvidia-smi reports GSP firmware N/A) and WILL TDR-loop on the unlocked card.
On Linux, GSP-RM is the default for Turing+ on 545+ drivers; N/A means the
driver is too old or NVreg_EnableGpuFirmware=0 is forced somewhere
(check: modprobe -c | grep -i EnableGpuFirmware). Fix the mode, RESTART
(not hibernate resume) and re-run. Building for another machine? Use
--skip-gsp-check."
                ;;
        esac
        GSP_GATE_STATUS="passed"
        log "[ok] host Turing adapter in GSP firmware mode"
    else
        log "[gate] nvidia-smi not found - no NVIDIA driver on THIS machine."
        log "       If the unlock target is a different machine, that is fine;"
        log "       otherwise install the NVIDIA driver first."
        prompt_continue || die "Aborted by user."
        GSP_GATE_STATUS="no-host-driver"
        log "[gate] skipped (no NVIDIA driver on this host)"
    fi
}

# ---------- main ----------
resolve_assets
if [ -z "$BOOT_EFI" ]; then BOOT_EFI="$SCRIPT_DIR/BOOTX64.EFI"; fi
[ -f "$BOOT_EFI" ] || die "BOOTX64.EFI not found: $BOOT_EFI
Build it from the repo (see README: 'cargo efi-tx' gated transaction build)
or pass --boot-efi."

mkdir -p "$OUT" "$WORK"
log "[assets] $ASSETS_BIN"
rtd3_notice
if [ "$FAMILY" = "tu10x" ]; then check_turing_gate; fi

fetch_verify() { # <name> <url> <sha> -> echoes dest path (logs go to stderr)
    local dest="$WORK/$1" part="$WORK/$1.part" got attempt
    for attempt in 1 2; do
        if [ ! -f "$dest" ]; then
            # Download to .part and rename only on success: a killed transfer used to
            # leave a truncated file at $dest, and the existence check then made every
            # later run report a pinned-input SHA-256 mismatch instead of re-fetching
            # (issue #59).
            rm -f "$part"
            if [ "$attempt" -eq 1 ]; then logerr "[fetch] $1"; else logerr "[fetch] $1 (retry)"; fi
            if ! fetch "$2" "$part"; then
                rm -f "$part"
                [ "$attempt" -eq 2 ] && die "download failed twice: $1 ($2)"
                continue
            fi
            mv -f "$part" "$dest"
        fi
        got="$(sha256_of "$dest")"
        if [ "$got" = "$3" ]; then
            logerr "[ok] $1 SHA-256 verified"
            printf '%s' "$dest"
            return 0
        fi
        # A stale local file must not masquerade as a pin mismatch; the hash still
        # gates the result, so re-fetching once is safe and self-heals the work dir.
        logerr "[warn] $1 hash mismatch on attempt $attempt - discarding and re-fetching"
        rm -f "$dest"
        if [ "$attempt" -eq 2 ]; then
            die "SHA-256 mismatch for $1: got $got want $3 - refusing (pinned inputs only)"
        fi
    done
    die "fetch failed: $1"
}

SRC_TAR="$(fetch_verify "$SRC1_NAME" "$SRC1_URL" "$SRC1_SHA")"
UTILS_ZST="$(fetch_verify "$SRC2_NAME" "$SRC2_URL" "$SRC2_SHA")"

# Remove assets from an earlier run so a root cannot end up as a half-old,
# half-new mix (e.g. a 40-series v67 payload next to a 20-series booter).
shopt -s nullglob
for f in "$OUT"/*; do
    [ -f "$f" ] || continue
    b="$(basename "$f")"
    if [[ "$b" =~ ^(booter_load_|gsp_|fwimage_|fwsignature_|sec2_bl_|scrubber_|v67-|gsp-gate\.txt$|BOOTX64\.EFI$) ]]; then
        rm -f -- "$f"
    fi
done
shopt -u nullglob

# ---------- per-family extraction ----------
expected=()
unlock_file=""
case "$FAMILY" in
    ad102)
        log "[extract] AD102 booter / GSP boot / scrubber"
        assets_run extract-ad102 "$SRC_TAR" "$OUT"
        assets_run extract-scrub "$SRC_TAR" "$OUT"
        log "[extract] GSP firmware images (gsp_ga10x)"
        assets_run fw "$UTILS_ZST" "$OUT"
        log "[extract] fwimage / fwsignature sections"
        assets_run gsp-sections "$OUT/gsp_ga10x.bin" "$OUT"
        log "[generate] V67 AD102 four-write payload"
        assets_run generate-ad102 "$OUT/v67-ad102-four-plm.bin"
        # gsp-sections emits the ELF section family name (ad10x); the loader
        # looks the device family up as ad102.
        cp -f "$OUT/fwimage_ad10x.bin" "$OUT/fwimage_ad102.bin"
        cp -f "$OUT/fwsignature_ad10x.bin" "$OUT/fwsignature_ad102.bin"
        expected=(booter_load_ad102.bin gsp_boot_desc_ad102.bin gsp_boot_ad102.bin
                  fwimage_ad102.bin fwsignature_ad102.bin scrubber_ad10x.bin
                  v67-ad102-four-plm.bin)
        unlock_file="v67-ad102-four-plm.bin"
        ;;
    ga102)
        log "[extract] GA102 booter / GSP boot / scrubber"
        assets_run extract "$SRC_TAR" "$OUT"
        assets_run extract-scrub "$SRC_TAR" "$OUT"
        log "[extract] GSP firmware images (gsp_ga10x)"
        assets_run fw "$UTILS_ZST" "$OUT"
        log "[extract] fwimage / fwsignature sections"
        assets_run gsp-sections "$OUT/gsp_ga10x.bin" "$OUT"
        log "[generate] V67 GA102 payload"
        assets_run generate "$OUT/v67-ga102-four-plm.bin"
        cp -f "$OUT/fwimage_ga10x.bin" "$OUT/fwimage_ga102.bin"
        cp -f "$OUT/fwsignature_ga10x.bin" "$OUT/fwsignature_ga102.bin"
        expected=(booter_load_ga102.bin gsp_boot_desc_ga102.bin gsp_boot_ga102.bin
                  fwimage_ga102.bin fwsignature_ga102.bin scrubber_ad10x.bin
                  v67-ga102-four-plm.bin)
        unlock_file="v67-ga102-four-plm.bin"
        ;;
    tu10x)
        log "[extract] TU102 booter (BOOT_DIRECT) / generic BL"
        assets_run extract-tu10x "$SRC_TAR" "$OUT"
        log "[extract] GSP firmware images (gsp_tu10x)"
        assets_run fw "$UTILS_ZST" "$OUT"
        log "[extract] fwimage / fwsignature sections"
        assets_run gsp-sections "$OUT/gsp_tu10x.bin" "$OUT"
        log "[generate] V67 TU10x payload"
        assets_run generate-tu10x "$OUT/v67-tu10x-four-plm.bin"

        # Resolve the proven GspRmBoot archive: explicit path, next to this
        # script, reference checkout, work dir, then the pinned URL.
        blob=""
        for cand in "$PROVEN_BLOB" \
                    "$SCRIPT_DIR/$PROVEN_NAME" \
                    "$SCRIPT_DIR/../reference/cmp50hx-unlock/efi-unlock/blobs/$PROVEN_NAME" \
                    "$WORK/$PROVEN_NAME"; do
            if [ -n "$cand" ] && [ -f "$cand" ]; then blob="$cand"; break; fi
        done
        if [ -z "$blob" ]; then
            blob="$WORK/$PROVEN_NAME"
            log "[fetch] $PROVEN_NAME (proven GspRmBoot archive)"
            fetch "$PROVEN_URL" "$blob"
        fi
        blob_hash="$(sha256_of "$blob")"
        [ "$blob_hash" = "$PROVEN_SRC_SHA" ] || die "proven GspRmBoot blob mismatch: got $blob_hash want $PROVEN_SRC_SHA ($blob) - refusing (the 128 KiB cmp50hx archive is the only accepted source; the gsp_tu10x.bin ELF is NOT it)"
        log "[ok] proven GspRmBoot archive SHA-256 verified ($(size_of "$blob") bytes)"
        log "[slice] first 0x6000 bytes -> gsp_boot_tu10x.bin"
        assets_run slice-proven-rm-boot "$blob" "$OUT"

        # The proven slice is byte-pinned: exactly 0x6000 bytes, de676bae... A
        # 4 KiB v4 BINDATA image or the GSP ELF's first 0x6000 (c76c43b1...)
        # must never ship.
        boot_out="$OUT/gsp_boot_tu10x.bin"
        len="$(size_of "$boot_out")"
        [ "$len" -eq "$PROVEN_SLICE_LEN" ] || die "gsp_boot_tu10x.bin is $len bytes, want $PROVEN_SLICE_LEN (0x6000): trimmed proven GspRmBoot slice only"
        boot_hash="$(sha256_of "$boot_out")"
        [ "$boot_hash" = "$PROVEN_SLICE_SHA" ] || die "gsp_boot_tu10x.bin hash $boot_hash != pinned $PROVEN_SLICE_SHA"
        log "[ok] gsp_boot_tu10x.bin = 0x6000 bytes, de676bae... (proven slice, byte-identical to the fired TU106 run)"

        expected=(booter_load_tu10x.bin gsp_boot_desc_tu10x.bin gsp_boot_tu10x.bin
                  fwimage_tu10x.bin fwsignature_tu10x.bin sec2_bl_tu10x.bin
                  v67-tu10x-four-plm.bin)
        unlock_file="v67-tu10x-four-plm.bin"
        {
            echo "nv-uefi-unlocker Turing GSP gate"
            echo "time: $(date -Is)"
            echo "host: $(hostname)"
            echo "family: tu10x"
            echo "gate: $GSP_GATE_STATUS"
            echo "adapters:"
            if [ -n "$ADAPTER_LINES" ]; then printf '%s\n' "$ADAPTER_LINES"; else echo "  (none detected on this host)"; fi
        } > "$OUT/gsp-gate.txt"
        ;;
esac

cp -f "$BOOT_EFI" "$OUT/BOOTX64.EFI"
expected+=("BOOTX64.EFI")

for f in "${expected[@]}"; do
    [ -f "$OUT/$f" ] || die "missing: $f"
    printf '  %-28s %10s bytes\n' "$f" "$(size_of "$OUT/$f")"
done

log ""
log "DONE ($FAMILY). Copy the CONTENTS of $OUT to a FAT32 USB stick root."
log "Format hint (Linux): sudo mkfs.fat -F 32 /dev/sdXN   # wipes the stick!"
log "Unlock mode requires $unlock_file on the stick;"
log "removing it switches the EFI into replay/diagnostic mode."
if [ "$FAMILY" = "tu10x" ]; then
    log ""
    log "Turing reminder: the TARGET machine must run GSP-RM firmware mode and"
    log "the unlock session must follow a RESTART - a hibernate resume does not"
    log "re-initialize the driver (gsp-gate.txt on the stick records this build)."
    log "Linux targets: GSP is on by default with 545+ drivers; also check the"
    log "README Linux note - RTD3 (DynamicPowerManagement) erases the unlock"
    log "after handoff (issue #9)."
fi
