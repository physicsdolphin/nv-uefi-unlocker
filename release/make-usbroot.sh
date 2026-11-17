#!/usr/bin/env bash
# make-usbroot.sh — bash twin of make-usbroot.ps1 (Linux / WSL / Git Bash).
#
# Regenerates the unlock USB stick root from pinned public inputs. NVIDIA
# firmware blobs are NOT distributed with this project; this script downloads
# the pinned public packages, verifies their SHA-256, and regenerates every
# file the EFI application needs for the chosen GPU family.
#
# Usage:
#   ./make-usbroot.sh                          (localhost: detect this machine's GPUs)
#   ./make-usbroot.sh --family tu10x --out /media/usb
#   ./make-usbroot.sh --family all             (every RTX 20/30/40 family)
#   ./make-usbroot.sh --family 30,40           (comma list - a mixed 30/40 rig
#                     needs exactly these two, without dragging Turing assets)
#
# Options (1:1 with the PowerShell twin's parameters):
#   --family F        ad102 | ga102 | tu10x | tu11x | all | localhost, or a
#                     comma/semicolon list of those tokens ("30,40"; list =
#                     exact set to build, the mixed-generation rig form)
#                     (aliases rtx40/40, rtx30/30, rtx20/20, rtx16/16;
#                     default localhost)
#                     localhost = enumerate the NVIDIA GPUs in THIS machine
#                     (nvidia-smi, else lspci) and build exactly the families
#                     found there; dies when no RTX 20/30/40/16 card is present.
#                     all = build all three 20-40 series families on one stick
#                     (tu11x excluded - explicit opt-in only).
#                     `all` anywhere wins; `localhost` inside a list is unioned
#                     with the explicit members; a list runs the Turing GSP gate
#                     strict, same as a single explicit family.
#   --out DIR         output root (default ./usbroot)
#   --work DIR        download/verify work dir (default ./inputs)
#   --assets PATH     assets pipeline binary. Resolution order: this flag /
#                     $ASSETS_BIN, then 'assets-x86_64-linux' or 'assets' next
#                     to this script (release zip), then an automatic
#                     `cargo build -p assets` in the enclosing repo checkout.
#   --boot-efi PATH   BOOTX64.EFI to place on the stick (default: next to script)
#   --proven-blob P   explicit 128 KiB cmp50hx GspRmBoot archive (tu10x only)
#   --utils-source S  nvidia-utils transport: auto (default: local driver
#                     firmware (badge mode) -> Arch archive -> first-party
#                     .run -> CachyOS cache, first that lands) or arch | run |
#                     cachyos | local to pin one route. Hash gate identical
#                     for downloads; local is badge mode.
#   --skip-gsp-check  build for another machine: skip the host Turing GSP check
#   --no-install-prompt  skip the post-build install-target offer
#   --install-to MP   copy the layout into an EXISTING mounted FAT32 partition
#                     without asking (same eligibility checks as the probe)
#   -h | --help       this text
#
# Family notes (same as make-usbroot.ps1):
#   ad102  RTX 40 series (Ada)          - no host prerequisites
#   ga102  RTX 30 series (Ampere)       - kmod-verified upstream, no silicon
#                                         evidence for this EFI path yet
#   tu10x  RTX 20 series (Turing)       - REQUIRES GSP-RM firmware mode on the
#                     TARGET machine; the legacy path TDR-loops on the
#                     unlocked card. On Linux the proprietary driver 545+
#                     runs GSP-RM by default on Turing and newer (no
#                     EnableGpuFirmware-style switch exists or is needed).
#                     The host gate therefore warns (never blocks) when
#                     nvidia-smi reports the GSP firmware N/A - driver too
#                     old, or NVreg_EnableGpuFirmware=0 forced in a modprobe
#                     config. Building for a DIFFERENT machine? Pass
#                     --skip-gsp-check.
#   tu11x  GTX 16 series (TU116/117)    - EXPERIMENTAL, same GSP gate.
#                     RE-derived route (issue #151): die-keyed TU116 booter
#                     archive + the TU102 frame table with the park word
#                     re-anchored (0x7EA4 -> 0x7E9A). NOT field-proven;
#                     read-back is the only verdict. Not part of --family all.
#   all    tu10x + ga102 + ad102 payloads on one stick. The Turing GSP gate
#                     runs in lenient mode: still warns when a LOCAL Turing
#                     card sits in legacy mode, but is silent when this host
#                     has no Turing adapter at all.
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

FAMILY="localhost"
OUT="./usbroot"
WORK="./inputs"
ASSETS_BIN="${ASSETS_BIN:-}"
BOOT_EFI=""
PROVEN_BLOB=""
SKIP_GSP_CHECK=0
# Post-build install offer (see the PowerShell twin's data-safety contract):
# probe EXISTING FAT32 partitions and offer to copy the layout into one.
NO_INSTALL_PROMPT=0
INSTALL_TO=""

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
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }
sha256_of() { sha256sum "$1" | awk '{print $1}'; }
size_of() { stat -c %s "$1"; }

fetch() { # fetch <url> <dest> -> nonzero on failure (caller decides next step)
    if have curl; then
        # Resume a partial $2 (HTTP Range) so a dropped 314 MB transfer
        # continues where it died instead of restarting from zero
        # (issues #52/#59/#180/#205). Exit 33 = the server refused the
        # resume range on a leftover .part: restart the transfer clean.
        curl -fsSL --retry 2 -C - --connect-timeout 30 -o "$2" "$1" && return 0
        [ "$?" -eq 33 ] || return 1
        curl -fsSL --retry 2 --connect-timeout 30 -o "$2" "$1"
    elif have wget; then
        # -c continues a partial file (wget restarts by itself when the
        # server has no range support); -t 3 retries, -T 30 bounds a stall.
        wget -qc -t 3 -T 30 -O "$2" "$1"
    else
        die "need curl or wget to download $1"
    fi
}

# ---------- pinned public inputs (see AGENTS.md / assets/src/manifest.rs) ----------
SRC1_NAME="NVIDIA-kernel-module-source-610.43.03.tar.xz"
SRC1_URL="https://download.nvidia.com/XFree86/NVIDIA-kernel-module-source/NVIDIA-kernel-module-source-610.43.03.tar.xz"
SRC1_SHA="7e118923c7a23edc36114d63273a46e3e04e9af98695a42203e7ac2dfe9fc1dc"

# The nvidia-utils GSP-firmware input is a SOURCE GROUP (issue #212): any
# container that yields the pinned inner ELFs is acceptable - the gate is
# content-addressed (gsp_ga10x 57237362..., gsp_tu10x 73065619...), not
# container-addressed. The original CachyOS v3 archive keeps only a
# ~3-version rolling window per package and dropped 610.43.03 on 2026-11-08;
# it stays last in line for anyone holding a cache of the exact bytes. The
# Arch Linux Archive (official rollback archive, versions never purged) and
# the first-party NVIDIA .run (XFree86 tree is append-only) are
# permanent-class transports; all three verified bit-identical firmware on
# 2026-11-08. Order: alive-and-permanent first. Fields: id|kind|name|sha|url
# (id feeds --utils-source; kind picks the extractor: pkg -> fw, run -> fw-run).
UTILS_SOURCES=(
    "arch|pkg|nvidia-utils-610.43.03-1-x86_64.pkg.tar.zst|c3367ff68632ac7daf6ca50ef7cbdd1cb5d09f7fe0b26125b312c73e52a34cf4|https://archive.archlinux.org/packages/n/nvidia-utils/nvidia-utils-610.43.03-1-x86_64.pkg.tar.zst"
    "run|run|NVIDIA-Linux-x86_64-610.43.03.run|45e2d4c134a23c35e50f253a4aa63e7e5e8d17e3d185d4a07c8a58e9612ed392|https://download.nvidia.com/XFree86/Linux-x86_64/610.43.03/NVIDIA-Linux-x86_64-610.43.03.run"
    "cachyos|pkg|nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst|e17e7c92eeffa178b0fe1f40eb17a47095f8fc3ce4a77bf115cf5bd5a4df86e9|https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst"
    "cachyos4|pkg|nvidia-utils-610.43.03-1-x86_64_v4.pkg.tar.zst|0fe26739363632a91df0d60738460afd5620abb3a9175f9fb4d8a3794d3db780|https://archive.cachyos.org/archive/cachyos-v4/nvidia-utils-610.43.03-1-x86_64_v4.pkg.tar.zst"
)

# Provenance-pinned Turing BOOT_DIRECT bootloader. 0x6000 is not an arbitrary
# truncation: it is GSP_RM_BOOT_SIZE (cmp50hx kernel source), the exact length
# the booter consumes. The slice (de676bae...) fired the successful TU106 run
# (2026-10-27, attempt 13: "gsp_boot_tu10x.bin loaded 24576/24576 bytes").
# The GSP ELF's first 0x6000 bytes (c76c43b1...) is NOT the same image and
# must not ship.
#
# Transport mirrors: the SHA-256 pin is the only authority - any source
# serving byte-identical bytes is acceptable, and a download failure never
# justifies a different version (constitution 6). raw.githubusercontent.com
# is DNS-poisoned/SNI-reset behind the Chinese mainland firewall, so the
# jsDelivr CDNs and a gh-proxy relay are pinned as verified transports for
# the SAME object (all three re-hashed to fa43239b... on 2026-11-01).
PROVEN_NAME="gsp_rm_boot_dbg.bin"
PROVEN_SRC_SHA="fa43239bcee7b97ca62f007cc68487560a39e19f74f3dde7486db3f98df8e471"
PROVEN_URLS=(
    "https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
    "https://fastly.jsdelivr.net/gh/xrip/cmp50hx-unlock@master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
    "https://cdn.jsdelivr.net/gh/xrip/cmp50hx-unlock@master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
    "https://gh-proxy.com/https://raw.githubusercontent.com/xrip/cmp50hx-unlock/master/efi-unlock/blobs/gsp_rm_boot_dbg.bin"
)
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
        # nvidia-utils transport: auto (default) walks the source group in
        # order (Arch archive -> first-party .run -> CachyOS cache); arch |
        # run | cachyos pins one route. Transport choice only - the hash
        # gate is identical, and an exact file already in $WORK wins.
        --utils-source)    UTILS_SOURCE="${2:?missing value for $1}"; shift 2 ;;
        --utils-source=*)  UTILS_SOURCE="${1#*=}"; shift ;;
        --skip-gsp-check)  SKIP_GSP_CHECK=1; shift ;;
        --no-install-prompt) NO_INSTALL_PROMPT=1; shift ;;
        --install-to)      INSTALL_TO="${2:?missing value for $1}"; shift 2 ;;
        --install-to=*)    INSTALL_TO="${1#*=}"; shift ;;
        -h|--help)         usage ;;
        *) die "unknown option: $1 (see --help)" ;;
    esac
done

FAMILY="${FAMILY,,}"

# ---------- device id -> build family (same ranges as loader/src/main.rs) ----------
family_of_dev() { # $1 = device id accepted by $(( )); echoes family or nothing
    local dv=$(( "$1" ))
    # Same split as the PowerShell twin's Get-UnlockFamily: tu10x is the
    # TU102/TU104/TU106 desktop range; 0x1F81..0x1FFF (TU116) and
    # 0x2182..0x21D1 (mobile TU117 + 21xx variants) are the 16-series ->
    # tu11x route (RE-derived, issue #151). Family 20 never produces tu11x.
    if   (( dv >= 0x1E02 && dv <= 0x1F80 )); then echo "tu10x"
    elif (( dv >= 0x1F81 && dv <= 0x1FFF )) || (( dv >= 0x2182 && dv <= 0x21D1 )); then echo "tu11x"
    elif (( dv >= 0x2200 && dv <= 0x22FF )) || (( dv >= 0x2400 && dv <= 0x25FF )); then echo "ga102"
    elif (( dv >= 0x2600 && dv <= 0x28FF )); then echo "ad102"
    fi
}

# ---------- localhost: enumerate THIS machine's NVIDIA GPUs ----------
detect_local_families() { # echoes the build families found here, one per line
    local rows="" dev fam line name has_tu=0 has_tu11=0 has_ga=0 has_ad=0
    if have nvidia-smi; then
        # pci.device_id prints device+vendor concatenated ("0x1F0710DE"):
        # strip 0x, keep the FIRST 4 hex digits (the device id).
        while IFS=, read -r dev name; do
            dev="${dev,,}"; dev="${dev// /}"; dev="${dev#*0x}"; dev="${dev:0:4}"
            [ "${#dev}" -eq 4 ] || continue
            rows+="${dev}|${name# }"$'\n'
        done < <(nvidia-smi --query-gpu=pci.device_id,name --format=csv,noheader 2>/dev/null || true)
    fi
    if [ -z "$rows" ] && have lspci; then
        # Fallback for driverless hosts: display-class PCI probes. Audio
        # functions on the GPU (10de:10f1 etc.) do not map to any family and
        # are ignored by the range check below.
        while IFS= read -r line; do
            dev="$(printf '%s\n' "$line" | grep -oE '\[10de:[0-9A-Fa-f]{4}\]' | head -n1)"
            [ -n "$dev" ] || continue
            dev="${dev%%]*}"; dev="${dev##*:}"
            rows+="${dev}|${line}"$'\n'
        done < <(lspci -nn -d 10de: 2>/dev/null | grep -Ei 'VGA |3D controller' || true)
    fi
    while IFS='|' read -r dev name; do
        [ -n "$dev" ] || continue
        fam="$(family_of_dev "0x$dev")"
        if [ -z "$fam" ]; then
            logerr "[localhost] 10de:$dev - outside RTX 20/30/40 series, ignoring (${name:-unknown device})"
            continue
        fi
        # Human-readable lines go to stderr: this function's stdout IS the
        # family list consumed by the caller.
        logerr "[localhost] 10de:$dev -> $fam (${name:-unknown device})"
        case "$fam" in
            tu10x) has_tu=1 ;;
            tu11x) has_tu11=1 ;;
            ga102) has_ga=1 ;;
            ad102) has_ad=1 ;;
        esac
    done <<< "$rows"
    [ "$has_tu" -eq 1 ] && echo "tu10x"
    [ "$has_tu11" -eq 1 ] && echo "tu11x"
    [ "$has_ga" -eq 1 ] && echo "ga102"
    [ "$has_ad" -eq 1 ] && echo "ad102"
    return 0
}

# ---------- resolve the family argument into a build list ----------
# --family takes one token or a comma/semicolon/space list ("30,40"); the list
# is the mixed-generation rig form. `all` anywhere wins; `localhost` inside a
# list is unioned with the explicit members. Alias normalization per token, then
# a canonical build order (same table the loader routes cards with).
GATE_STRICT=1
FAMILIES=()
WANT_ALL=0
WANT_LOCAL=0
WANTED=()
IFS=',; ' read -r -a _fam_tokens <<< "$FAMILY"
for _tok in "${_fam_tokens[@]}"; do
    [ -n "$_tok" ] || continue
    _tok="${_tok,,}"
    case "$_tok" in
        ad102|rtx40|40) _fam=ad102 ;;
        ga102|rtx30|30) _fam=ga102 ;;
        tu10x|rtx20|20) _fam=tu10x ;;
        tu11x|rtx16|16) _fam=tu11x ;;
        all) WANT_ALL=1; continue ;;
        localhost) WANT_LOCAL=1; continue ;;
        *) die "unknown family: '$_tok' (ad102 | ga102 | tu10x | tu11x | all | localhost, or a comma list such as 30,40)" ;;
    esac
    case " ${WANTED[*]-} " in *" $_fam "*) ;; *) WANTED+=("$_fam") ;; esac
done
if [ "${#WANTED[@]}" -eq 0 ] && [ "$WANT_ALL" -eq 0 ] && [ "$WANT_LOCAL" -eq 0 ]; then
    WANT_LOCAL=1
fi
if [ "$WANT_ALL" -eq 1 ]; then
    # One stick with every 20-40 series family. May target a machine
    # other than this one, so the Turing GSP gate runs lenient below.
    FAMILIES=(tu10x ga102 ad102)
    GATE_STRICT=0
else
    if [ "$WANT_LOCAL" -eq 1 ]; then
        _detected=()
        while IFS= read -r fam; do
            _detected+=("$fam")
        done < <(detect_local_families)
        if [ "${#_detected[@]}" -eq 0 ]; then
            if [ "${#WANTED[@]}" -eq 0 ]; then
                die "localhost: no supported NVIDIA adapter (RTX 20/30/40/16 series) found on
this machine. Use an explicit --family (ad102 | ga102 | tu10x | tu11x) or --family all."
            fi
            log "[localhost] no local supported adapter - using the explicit list only"
        fi
        for fam in "${_detected[@]}"; do
            case " ${WANTED[*]-} " in *" $fam "*) ;; *) WANTED+=("$fam") ;; esac
        done
        log "[localhost] building for: ${WANTED[*]-}"
        log "[localhost] full platform snapshot (board/BIOS/VBIOS/driver + advisories): python3 tools/diag-unlock.py --inventory"
        if [ "${#WANTED[@]}" -lt 3 ]; then
            log "[localhost] NOTE: this stick will only carry ${WANTED[*]-} assets. Cards from other generations in the TARGET machine (e.g. a mixed 30/40-series rig) will be skipped at boot. For a multi-generation machine use the comma list (30,40) or --family all"
        fi
    elif [ "${#WANTED[@]}" -gt 1 ]; then
        log "[family] explicit list - building: ${WANTED[*]}"
    fi
    for fam in tu10x tu11x ga102 ad102; do
        case " ${WANTED[*]-} " in *" $fam "*) FAMILIES+=("$fam") ;; esac
    done
fi

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

# ---------- Turing GSP-mode gate (warn without GSP firmware mode) ----------
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

check_turing_gate() { # $1 = 1 strict (explicit/localhost tu10x) | 0 lenient (all)
    local strict="${1:-1}"
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
            if [ "$strict" -eq 1 ]; then
                warn "[gate] No NVIDIA Turing adapter found on THIS machine."
                log "       If the unlock target is a different machine, pass --skip-gsp-check to silence this check."
            else
                # --family all may target a machine other than this one.
                log "[gate] no Turing adapter on this host - GSP check not applicable (--family all)"
            fi
            GSP_GATE_STATUS="no-turing"
            log "[gate] skipped (no Turing adapter on this host)"
            return 0
        fi

        # Runtime corroboration: a module parameter only proves the switch is
        # SET, not that the current driver session actually loaded GSP firmware
        # (a stale pre-restart session keeps the legacy path). On Linux the
        # nvidia-smi runtime field IS the primary check. Warn-and-continue:
        # the build host's state must not block stick regeneration.
        local state
        state="$(gsp_state)"
        case "$state" in
            active)
                log "[ok] GSP firmware running (nvidia-smi reports a firmware version)"
                GSP_GATE_STATUS="passed"
                log "[ok] host Turing adapter in GSP firmware mode"
                ;;
            failed)
                warn "nvidia-smi -q failed (driver present but not reporting?) - build continues, runtime GSP state unverified."
                GSP_GATE_STATUS="smi-failed"
                ;;
            unknown)
                warn "nvidia-smi has no 'GSP Firmware Version' field: the driver is too old to
report (or to support) GSP-RM. The build continues, but the Turing target
needs GSP firmware mode: update the driver (545+ runs GSP by default on
Turing+) and RESTART before using the stick."
                GSP_GATE_STATUS="runtime-unknown"
                ;;
            inactive)
                warn "Turing adapter(s) on this host are NOT in GSP firmware mode (nvidia-smi
reports GSP firmware N/A) and WILL TDR-loop on the unlocked card. The build
continues. On Linux, GSP-RM is the default for Turing+ on 545+ drivers; N/A
means the driver is too old or NVreg_EnableGpuFirmware=0 is forced somewhere
(check: modprobe -c | grep -i EnableGpuFirmware). Fix the mode and RESTART
(not hibernate resume) before using the stick."
                GSP_GATE_STATUS="runtime-inactive"
                ;;
        esac
    else
        log "[gate] nvidia-smi not found - no NVIDIA driver on THIS machine."
        log "       If the unlock target is a different machine, that is fine;"
        log "       otherwise install the NVIDIA driver first."
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
# Turing GSP gate: strict for an explicit/localhost tu10x build (refuse on a
# legacy-mode local card), lenient for --family all (may target another box).
RUN_TURING_GATE=0
for fam in "${FAMILIES[@]}"; do
    if [ "$fam" = "tu10x" ] || [ "$fam" = "tu11x" ]; then RUN_TURING_GATE=1; fi
done
if [ "$RUN_TURING_GATE" -eq 1 ]; then check_turing_gate "$GATE_STRICT"; fi

fetch_verify() { # <name> <sha> <url> [url...] -> sets FETCH_DEST on success;
    # returns 1 with FAIL_KIND/FAIL_LAST set on total failure (no die: the
    # caller decides - single sources die with the verdict, the utils source
    # group falls through to the next container). Must run in the current
    # shell so the globals survive - no $(...) capture.
    local dest="$WORK/$1" part="$WORK/$1.part" got pass url
    local name="$1" sha="$2"
    shift 2
    # A file already in the work dir with the pinned hash is accepted as-is:
    # that is the sanctioned offline path (obtain the exact bytes by any
    # transport, drop them here, build).
    if [ -f "$dest" ] && [ "$(sha256_of "$dest")" = "$sha" ]; then
        logerr "[ok] $name SHA-256 verified (cached)"
        FETCH_DEST="$dest"
        return 0
    fi
    rm -f "$dest"
    # The SHA-256 pin is the authority, not the URL: try every transport.
    # Download to .part and hash it there - a killed or corrupted transfer
    # never lands at $dest (issue #59). A partial .part is KEPT across
    # attempts so the next pass (or the next run) resumes it. Two DISTINCT
    # failure verdicts: a transport that never delivered is not a hash
    # accusation, and wrong bytes are not flaky network (issue #205).
    local served=0 had last="no transport attempted"
    for pass in 1 2 3; do
        for url in "$@"; do
            had="$(size_of "$part" 2>/dev/null || echo 0)"
            if [ "$had" -gt 0 ]; then
                logerr "[fetch] $name <- ${url#https://} (pass $pass, resuming $had bytes)"
            else
                logerr "[fetch] $name <- ${url#https://} (pass $pass)"
            fi
            if fetch "$url" "$part"; then
                got="$(sha256_of "$part")"
                if [ "$got" = "$sha" ]; then
                    mv -f "$part" "$dest"
                    logerr "[ok] $name SHA-256 verified"
                    FETCH_DEST="$dest"
                    return 0
                fi
                served=1
                last="wrong bytes from ${url#https://} (sha256 $got)"
                logerr "[warn] $name: ${url#https://} served the wrong bytes (sha256 $got, want $sha) - refusing this copy"
                # Wrong bytes must never be resumed into a good file.
                rm -f "$part"
            else
                had="$(size_of "$part" 2>/dev/null || echo 0)"
                logerr "[warn] $name: transport failed from ${url#https://}; $had bytes kept for resume"
                last="transport failure from ${url#https://}"
            fi
        done
    done
    rm -f "$part"
    FAIL_KIND="no-transport"; FAIL_LAST="$last"
    [ "$served" -eq 1 ] && FAIL_KIND="wrong-bytes"
    return 1
}

# Resolve the nvidia-utils source group: automatic fallback, the first
# container that lands wins (cached file or verified download). Sets
# UTILS_PATH + UTILS_KIND (pkg -> fw, run -> fw-run). On total failure dies
# with the all-sources report: every transport, its pinned hash, and the
# fully-offline option (any ONE of the files).
resolve_utils() {
    local entry id kind name sha url
    case "${UTILS_SOURCE:-auto}" in
        auto|arch|run|cachyos|cachyos4|local) ;;
        *) die "invalid --utils-source '$UTILS_SOURCE' (auto | arch | run | cachyos | local)" ;;
    esac
    # local: the host's own installed driver firmware (badge mode,
    # docs/gsp-compatibility.md - version pin waived, measured 525-621
    # compatible). nvidia-utils installs the split-format ELFs under
    # /usr/lib/firmware/nvidia/<ver>/ (also /lib/firmware on some distros);
    # the highest version wins. In auto mode this leads; downloads follow.
    if { [ "${UTILS_SOURCE:-auto}" = "auto" ] || [ "${UTILS_SOURCE:-auto}" = "local" ]; }; then
        local fwdir=""
        # The extra globs cover Windows-hosted bash: the MSYS/WSL mapped
        # DriverStore carries the same split gsp_*.bin ELFs as a Linux
        # nvidia-utils install (Git Bash: /c/..., WSL: /mnt/c/...) - on a
        # real Linux box they simply do not exist and the glob stays a
        # literal the -f test rejects. This is what keeps --family working
        # on a Windows host without a download.
        # NB the `if` form is load-bearing: a `&& echo` chain leaves the
        # loop's exit status nonzero when the LAST glob does not match,
        # and pipefail + set -e then kills the whole script silently
        # (the assignment fails) before any download is attempted.
        fwdir=$(for d in /usr/lib/firmware/nvidia/* /lib/firmware/nvidia/*                     /run/current-system/firmware/nvidia/* /var/run/current-system/firmware/nvidia/*                     /c/Windows/System32/DriverStore/FileRepository/nv*inf_amd64_*                     /mnt/c/Windows/System32/DriverStore/FileRepository/nv*inf_amd64_*; do
            if [ -f "$d/gsp_tu10x.bin" ] && [ -f "$d/gsp_ga10x.bin" ]; then echo "$d"; fi
        done | sort -V | tail -n1)
        if [ -n "$fwdir" ]; then
            UTILS_PATH="$fwdir"
            UTILS_KIND="local"
            log "[local] GSP from the installed driver: $fwdir (badge mode, version pin waived)"
            return 0
        fi
        log "[local] no installed-driver GSP found (/usr/lib|/lib|/run/current-system)/firmware/nvidia/*/gsp_*.bin and the Windows DriverStore are all absent)"
        [ "${UTILS_SOURCE:-auto}" = "local" ] && die "--utils-source local: no installed-driver GSP found"
    fi
    for entry in "${UTILS_SOURCES[@]}"; do
        IFS='|' read -r id kind name sha url <<< "$entry"
        [ "${UTILS_SOURCE:-auto}" = "auto" ] || [ "$id" = "$UTILS_SOURCE" ] || continue
        if fetch_verify "$name" "$sha" "$url"; then
            UTILS_PATH="$FETCH_DEST"
            UTILS_KIND="$kind"
            return 0
        fi
        log "[fallback] $name unusable ($FAIL_KIND: $FAIL_LAST) - trying the next pinned source"
    done
    {
        echo "ERROR: ALL pinned nvidia-utils sources failed - the GSP firmware container could not be obtained."
        echo "Every attempt above logs its own verdict (transport failure vs wrong bytes). Sources tried:"
        [ "${UTILS_SOURCE:-auto}" != "auto" ] && echo "Only '$UTILS_SOURCE' was tried (pinned via --utils-source). Re-run without the flag to try every transport."
        for entry in "${UTILS_SOURCES[@]}"; do
            IFS='|' read -r id kind name sha url <<< "$entry"
            echo "  - $name"
            echo "      $url"
            echo "      sha256 $sha"
        done
        echo "Options:"
        echo "  1. Fix the network condition and re-run - a partial download resumes where it died."
        echo "  2. Go offline: download ANY ONE of the files above by any means (browser, another"
        echo "     machine, proxy) and drop it into $WORK/ unchanged - the SHA-256 gate accepts it"
        echo "     offline. All three carry bit-identical GSP firmware."
        echo "Never substitute a different driver version: the pinned SHA-256 values above are the"
        echo "only accepted bytes (a newer nvidia-utils, e.g. 615.x, is NOT interchangeable)."
    } >&2
    exit 1
}

if fetch_verify "$SRC1_NAME" "$SRC1_SHA" "$SRC1_URL"; then
    SRC_TAR="$FETCH_DEST"
else
    if [ "$FAIL_KIND" = "wrong-bytes" ]; then
        die "download failed for $SRC1_NAME: a source answered but did not serve the pinned bytes (sha256 $SRC1_SHA). The pin/URL pairing is broken - please report this. Never substitute another version of the file."
    fi
    die "download failed for $SRC1_NAME: no transport delivered the file (last: $FAIL_LAST).
This is a network/client failure, not a bad pin - re-run to retry (a partial download resumes), or obtain the exact file by any means and drop it into $WORK/ (sha256 $SRC1_SHA) to build offline."
fi
resolve_utils

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

# ---------- shared extraction (one pass for the whole build) ----------
# fw / fw-run always extract BOTH GSP ELFs (gsp_ga10x.bin + gsp_tu10x.bin)
# in a single pass, so it runs once no matter how many families build. The
# subcommand follows whichever source-group container landed: a .pkg.tar.zst
# goes through fw, the first-party .run through fw-run (same inner hash
# gate; requires an assets build that advertises fw-run in its usage).
log "[extract] GSP firmware images (gsp_ga10x + gsp_tu10x)"
if [ "$UTILS_KIND" = "local" ]; then
    # Badge mode (docs/gsp-compatibility.md): the ELFs come from the host's
    # installed driver (fwimage+fwsignature paired by construction). Version
    # pin WAIVED - measured 525-621 all unlock. Format-gated downstream: the
    # per-family gsp-sections calls fail loudly on a pre-52x single-ELF store.
    log "[extract] BADGE MODE: GSP from the local installed driver ($UTILS_PATH) - version pin waived (525-621 measured compatible)"
    cp "$UTILS_PATH/gsp_ga10x.bin" "$UTILS_PATH/gsp_tu10x.bin" "$OUT/"
    log "[ok] gsp_ga10x.bin + gsp_tu10x.bin staged from the local driver (format gate = per-family gsp-sections)"
elif [ "$UTILS_KIND" = "run" ]; then
    if ! "$ASSETS_BIN" 2>&1 | grep -q "fw-run"; then
        die "the nvidia-utils input is the first-party .run, but this assets build has no fw-run support. Rebuild assets (cargo build -p assets) or supply either .pkg.tar.zst container from the source group instead."
    fi
    assets_run fw-run "$UTILS_PATH" "$OUT"
else
    assets_run fw "$UTILS_PATH" "$OUT"
fi

needs_scrub=0
for fam in "${FAMILIES[@]}"; do
    # Same exclusion set as the PowerShell twin: the Turing families
    # (tu10x AND tu11x) run no scrubber step in the proven chain.
    if [ "$fam" != "tu10x" ] && [ "$fam" != "tu11x" ]; then needs_scrub=1; fi
done
if [ "$needs_scrub" -eq 1 ]; then
    log "[extract] SEC2 scrubber (shared: ad102/ga102)"
    assets_run extract-scrub "$SRC_TAR" "$OUT"
fi

# ---------- per-family extraction ----------
expected=()
unlock_files=()
ga10x_sections_done=0

# gsp-sections exports fwimage_<fam>.bin + fwsignature_<fam>.bin per wanted
# signature family. The .fwimage section is family-neutral (the same tens of
# MB get rewritten for every exported name), so only the families this build
# serves are requested - the ga10x ELF carries ga100/ga10x/ad10x signatures,
# the tu10x ELF tu10x/tu11x/ga100. The loader looks families up by device
# family name, hence the ad10x->ad102 / ga10x->ga102 copies below.
GA_SIG_FAMILIES=""
TU_SIG_FAMILIES=""
for fam in "${FAMILIES[@]}"; do
    case "$fam" in
        ga102) GA_SIG_FAMILIES="$GA_SIG_FAMILIES ga10x" ;;
        ad102) GA_SIG_FAMILIES="$GA_SIG_FAMILIES ad10x" ;;
        tu10x) TU_SIG_FAMILIES="$TU_SIG_FAMILIES tu10x" ;;
        tu11x) TU_SIG_FAMILIES="$TU_SIG_FAMILIES tu11x" ;;
    esac
done
# shellcheck disable=SC2086  # controlled single-word tokens, word-split on purpose
GA_SIG_FAMILIES="${GA_SIG_FAMILIES# }"
TU_SIG_FAMILIES="${TU_SIG_FAMILIES# }"

# Resolve the proven GspRmBoot archive (Turing-shared, tu10x AND tu11x use
# the same pinned bytes): explicit path, next to this script, reference
# checkout, work dir, then the pinned URL. Sets PROVEN_BLOB_PATH.
resolve_proven_blob() {
    PROVEN_BLOB_PATH=""
    local cand
    for cand in "$PROVEN_BLOB" \
                "$SCRIPT_DIR/$PROVEN_NAME" \
                "$SCRIPT_DIR/../reference/cmp50hx-unlock/efi-unlock/blobs/$PROVEN_NAME" \
                "$WORK/$PROVEN_NAME"; do
        if [ -n "$cand" ] && [ -f "$cand" ]; then PROVEN_BLOB_PATH="$cand"; return 0; fi
    done
    # Same transport + hash gate as the pinned sources above; falls through
    # to the not-found die with its own offline hint when every transport
    # fails.
    fetch_verify "$PROVEN_NAME" "$PROVEN_SRC_SHA" "${PROVEN_URLS[@]}" || true
    PROVEN_BLOB_PATH="$WORK/$PROVEN_NAME"
}

# Slice + byte-pin check for one Turing family's gsp_boot_<fam>.bin. The
# proven slice is byte-pinned: exactly 0x6000 bytes, de676bae... A 4 KiB v4
# BINDATA image or the GSP ELF's first 0x6000 (c76c43b1...) must never ship.
proven_slice() { # $1 = family
    local fam="$1" len hash
    resolve_proven_blob
    local blob="$PROVEN_BLOB_PATH"
    [ -f "$blob" ] || die "could not fetch $PROVEN_NAME from any pinned transport; drop the exact file (sha256 $PROVEN_SRC_SHA) into $WORK/ and re-run"
    hash="$(sha256_of "$blob")"
    [ "$hash" = "$PROVEN_SRC_SHA" ] || die "proven GspRmBoot blob mismatch: got $hash want $PROVEN_SRC_SHA ($blob) - refusing (the 128 KiB cmp50hx archive is the only accepted source; the gsp_tu10x.bin ELF is NOT it)"
    log "[ok] proven GspRmBoot archive SHA-256 verified ($(size_of "$blob") bytes)"
    log "[slice] first 0x6000 bytes -> gsp_boot_$fam.bin"
    assets_run slice-proven-rm-boot "$blob" "$OUT" "$fam"
    len="$(size_of "$OUT/gsp_boot_$fam.bin")"
    [ "$len" -eq "$PROVEN_SLICE_LEN" ] || die "gsp_boot_$fam.bin is $len bytes, want $PROVEN_SLICE_LEN (0x6000): trimmed proven GspRmBoot slice only"
    hash="$(sha256_of "$OUT/gsp_boot_$fam.bin")"
    [ "$hash" = "$PROVEN_SLICE_SHA" ] || die "gsp_boot_$fam.bin hash $hash != pinned $PROVEN_SLICE_SHA"
    log "[ok] gsp_boot_$fam.bin = 0x6000 bytes, de676bae... (proven slice, byte-identical to the fired TU106 run)"
}

write_gate_record() { # $1 = family line for gsp-gate.txt
    {
        echo "nv-uefi-unlocker Turing GSP gate"
        echo "time: $(date -Is)"
        echo "host: $(hostname)"
        echo "family: $1"
        echo "gate: $GSP_GATE_STATUS"
        echo "adapters:"
        if [ -n "$ADAPTER_LINES" ]; then printf '%s\n' "$ADAPTER_LINES"; else echo "  (none detected on this host)"; fi
    } > "$OUT/gsp-gate.txt"
}

for fam in "${FAMILIES[@]}"; do
    case "$fam" in
        ad102)
            log "[extract] AD102 booter / GSP boot"
            assets_run extract-ad102 "$SRC_TAR" "$OUT"
            if [ "$ga10x_sections_done" -eq 0 ]; then
                log "[extract] fwimage / fwsignature sections (ga10x ELF: $GA_SIG_FAMILIES)"
                # shellcheck disable=SC2086  # controlled single-word tokens
                assets_run gsp-sections "$OUT/gsp_ga10x.bin" "$OUT" $GA_SIG_FAMILIES
                ga10x_sections_done=1
            fi
            log "[generate] V67 AD102 four-write payload"
            assets_run generate-ad102 "$OUT/v67-ad102-four-plm.bin"
            # gsp-sections emits the ELF section family name (ad10x); the loader
            # looks the device family up as ad102.
            cp -f "$OUT/fwimage_ad10x.bin" "$OUT/fwimage_ad102.bin"
            cp -f "$OUT/fwsignature_ad10x.bin" "$OUT/fwsignature_ad102.bin"
            expected+=(booter_load_ad102.bin gsp_boot_desc_ad102.bin gsp_boot_ad102.bin
                       fwimage_ad102.bin fwsignature_ad102.bin scrubber_ad10x.bin
                       v67-ad102-four-plm.bin)
            unlock_files+=("v67-ad102-four-plm.bin")
            ;;
        ga102)
            log "[extract] GA102 booter / GSP boot"
            assets_run extract "$SRC_TAR" "$OUT"
            if [ "$ga10x_sections_done" -eq 0 ]; then
                log "[extract] fwimage / fwsignature sections (ga10x ELF: $GA_SIG_FAMILIES)"
                # shellcheck disable=SC2086  # controlled single-word tokens
                assets_run gsp-sections "$OUT/gsp_ga10x.bin" "$OUT" $GA_SIG_FAMILIES
                ga10x_sections_done=1
            fi
            log "[generate] V67 GA102 payload"
            assets_run generate "$OUT/v67-ga102-four-plm.bin"
            cp -f "$OUT/fwimage_ga10x.bin" "$OUT/fwimage_ga102.bin"
            cp -f "$OUT/fwsignature_ga10x.bin" "$OUT/fwsignature_ga102.bin"
            expected+=(booter_load_ga102.bin gsp_boot_desc_ga102.bin gsp_boot_ga102.bin
                       fwimage_ga102.bin fwsignature_ga102.bin scrubber_ad10x.bin
                       v67-ga102-four-plm.bin)
            unlock_files+=("v67-ga102-four-plm.bin")
            ;;
        tu10x)
            log "[extract] TU102 booter (BOOT_DIRECT) / generic BL"
            assets_run extract-tu10x "$SRC_TAR" "$OUT"
            log "[extract] fwimage / fwsignature sections (tu10x ELF: $TU_SIG_FAMILIES)"
            # shellcheck disable=SC2086  # controlled single-word tokens
            assets_run gsp-sections "$OUT/gsp_tu10x.bin" "$OUT" $TU_SIG_FAMILIES
            log "[generate] V67 TU10x payload"
            assets_run generate-tu10x "$OUT/v67-tu10x-four-plm.bin"

            proven_slice tu10x

            expected+=(booter_load_tu10x.bin gsp_boot_desc_tu10x.bin gsp_boot_tu10x.bin
                       fwimage_tu10x.bin fwsignature_tu10x.bin sec2_bl_tu10x.bin
                       v67-tu10x-four-plm.bin)
            unlock_files+=("v67-tu10x-four-plm.bin")
            write_gate_record tu10x
            ;;
        tu11x)
            log "[warn] TU11x route (GTX 16 series / CMP 30HX) is RE-derived and NOT"
            log "[warn] field-proven (issue #151): die-keyed TU116 booter archive + the"
            log "[warn] TU102 frame table with the park word re-anchored (0x7EA4 -> 0x7E9A)."
            log "[warn] Read-back is the only verdict; a clean failure is an expected outcome."
            log "[extract] TU116/TU117 booter (BOOT_DIRECT, die-keyed) / shared Turing BL"
            assets_run extract-tu11x "$SRC_TAR" "$OUT"
            log "[extract] fwimage / fwsignature sections (tu10x ELF: $TU_SIG_FAMILIES)"
            # shellcheck disable=SC2086  # controlled single-word tokens
            assets_run gsp-sections "$OUT/gsp_tu10x.bin" "$OUT" $TU_SIG_FAMILIES
            log "[generate] V67 TU11x payload (RE-derived, unproven)"
            assets_run generate-tu11x "$OUT/v67-tu11x-four-plm.bin"

            # The proven GspRmBoot archive is Turing-shared (the driver
            # generates only a TU102 one), so tu11x slices the same bytes.
            proven_slice tu11x

            expected+=(booter_load_tu11x.bin gsp_boot_desc_tu11x.bin gsp_boot_tu11x.bin
                       fwimage_tu11x.bin fwsignature_tu11x.bin sec2_bl_tu11x.bin
                       v67-tu11x-four-plm.bin)
            unlock_files+=("v67-tu11x-four-plm.bin")
            write_gate_record tu11x
            ;;
    esac
done

cp -f "$BOOT_EFI" "$OUT/BOOTX64.EFI"
expected+=("BOOTX64.EFI")

# ---------- intermediates off the root ----------
# The full GSP ELFs (gsp_ga10x.bin / gsp_tu10x.bin) are extraction
# INTERMEDIATES: gsp-sections has consumed them and the loader reads only
# fwimage_*/fwsignature_*/booter_load_*/... from the volume root - the ELF
# files (84 + 29 MB) would be pure dead weight on the stick.
for elf in gsp_ga10x.bin gsp_tu10x.bin; do
    [ -f "$OUT/$elf" ] && rm -f -- "$OUT/$elf"
done

# Firmware only auto-boots \EFI\BOOT\BOOTX64.EFI; a flat copy of $OUT leaves the
# executable at the root and the boot menu bounces back (issue #142).
mkdir -p "$OUT/EFI/BOOT"
cp -f "$BOOT_EFI" "$OUT/EFI/BOOT/BOOTX64.EFI"

for f in "${expected[@]}"; do
    [ -f "$OUT/$f" ] || die "missing: $f"
    printf '  %-28s %10s bytes\n' "$f" "$(size_of "$OUT/$f")"
done
printf '  %-28s %10s bytes\n' "EFI/BOOT/BOOTX64.EFI" "$(size_of "$OUT/EFI/BOOT/BOOTX64.EFI")"

# Layout marker: identifies a volume that already hosts THIS project's files
# so a later run may upgrade them in place. Any other existing file is never
# overwritten (data-safety contract; see the install offer at the end).
printf '{\n  "project": "nv-uefi-unlocker",\n  "maker": "make-usbroot.sh",\n  "families": "%s",\n  "built": "%s"\n}\n' \
    "${FAMILIES[*]}" "$(date -Is)" > "$OUT/nvuu-stick.json"

# \dxe-manifest.txt (PR #100 audit M1): sha256sum-style digests of the
# asset set. The firmware-resident DXE driver form refuses a transaction
# whose assets do not match this manifest (fail-closed); the application
# form ignores the file. Backslash paths match the loader's probes.
: > "$OUT/dxe-manifest.txt"
for f in "${expected[@]}"; do
    case "$f" in
        BOOTX64.EFI|gsp-gate.txt) continue ;;
    esac
    printf '%s  \%s\n' "$(sha256_of "$OUT/$f")" "$f" >> "$OUT/dxe-manifest.txt"
done
expected+=("dxe-manifest.txt")

log ""
if mountpoint -q "$OUT" 2>/dev/null; then
    log "DONE (${FAMILIES[*]}). $OUT already IS a filesystem root - nothing to copy, boot it directly."
else
    log "DONE (${FAMILIES[*]}). The stick root must receive the layout; for example:"
    log "  cp -a \"$OUT\"/. /media/\$USER/<stick>/      # or: sudo cp -a \"$OUT\"/. /mnt/<mount>/"
    log "Or re-run with --out <mountpoint> to write a mounted stick directly."
fi
log "The layout already contains EFI/BOOT/BOOTX64.EFI - keep that path, do not flatten it."
log "Format hint (Linux): sudo mkfs.fat -F 32 /dev/sdXN   # wipes the stick!"
log "Unlock mode requires the v67-*.bin payload for the target card's family"
log "(${unlock_files[*]}); removing it switches the EFI into replay/diagnostic"
log "mode for that family."
if [ "$RUN_TURING_GATE" -eq 1 ]; then
    log ""
    log "Turing reminder: the TARGET machine must run GSP-RM firmware mode and"
    log "the unlock session must follow a RESTART - a hibernate resume does not"
    log "re-initialize the driver (gsp-gate.txt on the stick records this build)."
    log "Linux targets: GSP is on by default with 545+ drivers; also check the"
    log "README Linux note - RTD3 (DynamicPowerManagement) erases the unlock"
    log "after handoff (issue #9)."
fi

# ---------- install-target probe + one-click persistent install ----------
# The generated layout can be copied into an EXISTING FAT32 partition so the
# unlock does not require a dedicated USB stick every time: an internal
# (non-removable) disk's non-boot data vfat partition is offered for a
# persistent install, and a USB stick's vfat partition for the same copy.
# Data-safety contract (the issue #73 ruling kept intact): this probe NEVER
# creates, resizes, reformats or repartitions anything, NEVER touches an
# ESP/system/boot volume, and NEVER overwrites or deletes an existing file -
# a differing file at the target refuses the whole target, and only a
# previous nv-uefi-unlocker layout (identified by nvuu-stick.json) may be
# upgraded in place. A layout on an internal partition boots through the
# firmware's ONE-TIME boot menu; no NVRAM/BootOrder entry is ever written.

INTERNAL_TARGETS=()   # "mountpoint|free|size|label|devnode"
USB_TARGETS=()

human_size() { # $1 = bytes
    awk -v b="$1" 'BEGIN {
        if (b >= 1073741824) printf "%.1f GB", b/1073741824;
        else if (b >= 1048576) printf "%.1f MB", b/1048576;
        else printf "%.0f KB", b/1024 }'
}

our_marker() { # $1 = mountpoint: is this a previous nv-uefi-unlocker layout?
    [ -f "$1/nvuu-stick.json" ] && grep -q "nv-uefi-unlocker" "$1/nvuu-stick.json" 2>/dev/null
}

install_copy() { # $1 = src root, $2 = target mountpoint -> 0 ok, 1 refused
    local src="$1" mp="$2" f rel dest bad=0 copied=0 skipped=0 total
    total="$(find "$src" -type f | wc -l)"
    # Pass 1 - conflict scan: a differing existing file refuses the whole
    # target unless nvuu-stick.json marks it as our own earlier layout.
    local conflict=0
    while IFS= read -r f; do
        rel="${f#"$src"/}"
        dest="$mp/$rel"
        [ -e "$dest" ] || continue
        cmp -s "$f" "$dest" && continue
        if our_marker "$mp"; then continue; fi   # upgrade in place
        conflict=1
        logerr "[install]   existing: $mp/$rel"
    done < <(find "$src" -type f)
    if [ "$conflict" -eq 1 ]; then
        logerr "[install] REFUSED: the target already holds different file(s) and is not a nv-uefi-unlocker layout."
        logerr "[install] nothing was written (the tool never overwrites existing files)."
        logerr "[install] If it is a stale copy of this tool, delete those files yourself and re-run."
        return 1
    fi
    # Pass 2 - copy. Identical files are skipped (nothing rewritten).
    while IFS= read -r f; do
        rel="${f#"$src"/}"
        dest="$mp/$rel"
        if [ -e "$dest" ]; then
            if cmp -s "$f" "$dest"; then skipped=$((skipped + 1)); continue; fi
        fi
        mkdir -p "$mp/$(dirname "$rel")" || return 1
        cp -f "$f" "$dest" || {
            logerr "[install] copy failed: $f -> $dest (target full or read-only?)"
            return 1
        }
        copied=$((copied + 1))
    done < <(find "$src" -type f)
    # Pass 3 - read-back is the only success criterion (file domain too).
    while IFS= read -r f; do
        rel="${f#"$src"/}"
        dest="$mp/$rel"
        if ! cmp -s "$f" "$dest" || [ "$(sha256_of "$f")" != "$(sha256_of "$dest")" ]; then
            logerr "[install] READ-BACK MISMATCH: $dest - do not boot from this copy"
            bad=1
        fi
    done < <(find "$src" -type f)
    [ "$bad" -eq 1 ] && return 1
    log "[install] done: $total files ($copied written, $skipped already identical) -> $mp (SHA-256 read-back verified)"
    return 0
}

install_guidance() { # $1 = needed bytes
    log "[install] no install target was picked (none found, or both offers declined)."
    log "[install]   Internal (persistent) install: add a data FAT32 partition on an"
    log "[install]     INTERNAL (non-USB) disk - NOT the EFI system partition, NOT any"
    log "[install]     /boot volume (gparted/fdisk: shrink a volume, add a FAT32 one;"
    log "[install]     500 MB is ample). This tool never creates, resizes or reformats"
    log "[install]     partitions - it only copies files into an existing one."
    log "[install]   USB install: any stick keeping at least $(human_size "$1") free on a"
    log "[install]     FAT32 partition; the files are added alongside its contents."
}

install_boot_hints() {
    log "[install] boot path: firmware ONE-TIME boot menu -> the UEFI entry for that"
    log "[install] volume -> \\EFI\\BOOT\\BOOTX64.EFI. No NVRAM/BootOrder entry was"
    log "[install] created; the unlock itself stays volatile by design and is replayed"
    log "[install] at every boot."
    log "[install] caveat: not every firmware lists internal FAT partitions in its boot"
    log "[install] menu - the USB stick remains the universal path."
}

# Build INTERNAL_TARGETS / USB_TARGETS from lsblk. Only MOUNTED, writable
# vfat partitions with room for the layout are offered; ESP partitions
# (PARTTYPENAME "EFI System"), /boot*/  /efi mounts and the $OUT mount are
# never candidates. Unmounted partitions are listed as hints, not offers
# (installing into one would require mounting it here - too invasive).
install_targets() { # $1 = needed bytes, $2 = excluded mountpoint
    INTERNAL_TARGETS=()
    USB_TARGETS=()
    if ! have lsblk; then
        log "[install] lsblk not found - install-target probe skipped"
        return 0
    fi
    local rows
    rows="$(lsblk -rno KNAME,RM,TYPE,FSTYPE,MOUNTPOINT,PARTTYPENAME,LABEL 2>/dev/null)" || rows=""
    if [ -z "$rows" ]; then
        # Distinguish "old lsblk" from "no block devices seen" (WSL2/container).
        if lsblk -rno KNAME,TYPE,FSTYPE 2>/dev/null | grep -q .; then
            log "[install] lsblk too old for PARTTYPENAME - cannot tell ESP from data, probe skipped"
        else
            log "[install] no block devices seen by lsblk (container/WSL2 host?) - probe skipped"
        fi
        return 0
    fi
    local need=$(( "$1" + 16777216 ))   # + 16 MB headroom over the layout size
    local unmounted="" kname rm ftype mp ptype label dev is_usb free first_mp
    while IFS=, read -r kname rm ftype mp ptype label; do
        [ "$ftype" = "vfat" ] || continue
        dev="/dev/$kname"
        first_mp="${mp%%,*}"
        if [ -z "$first_mp" ] || [ "$first_mp" = "swap" ]; then
            unmounted+="  $dev  (not mounted - mount it first: udisksctl mount -b $dev)"$'\n'
            continue
        fi
        # ESP is constitution territory (never written, not even appended to).
        if [ "$ptype" = "EFI System" ]; then continue; fi
        case "$first_mp" in
            /|/boot|/boot/*|/efi|/efi/*|/bootx64efi) continue ;;   # boot volumes
        esac
        [ "$first_mp" = "$2" ] && continue   # the mount $OUT itself lives on
        if ! [ -w "$first_mp" ]; then
            unmounted+="  $dev  $first_mp (not writable by this user - try sudo or polkit mount)"$'\n'
            continue
        fi
        free="$(df -B1 --output=avail "$first_mp" 2>/dev/null | tail -n1 || true)"
        [ -n "$free" ] || continue
        if [ "$free" -lt "$need" ]; then continue; fi
        is_usb=0
        if [ "$rm" = "1" ]; then is_usb=1; fi
        if [ "$is_usb" -eq 0 ] && have udevadm && udevadm info -q property -n "$dev" 2>/dev/null | grep -q '^ID_BUS=usb'; then
            is_usb=1
        fi
        local size
        size="$(df -B1 --output=size "$first_mp" 2>/dev/null | tail -n1 || true)"
        if [ "$is_usb" -eq 1 ]; then
            USB_TARGETS+=("$first_mp|$free|$size|$label|$dev")
        else
            INTERNAL_TARGETS+=("$first_mp|$free|$size|$label|$dev")
        fi
    done <<< "$rows"
    if [ -n "$unmounted" ]; then
        log "[install] vfat partitions seen but not offered (not mounted / not writable):"
        printf '%s' "$unmounted"
    fi
    return 0
}

install_offer() {
    local needed_bytes out_mount ans n picked_entry mp free size label dev
    local entries=()
    # `|| true` inside the substitutions: under pipefail + set -e a failing
    # df/du (MSYS df has no --output; a missing volume) would otherwise kill
    # the script silently - the empty result is handled below.
    needed_bytes="$(du -sb "$OUT" 2>/dev/null | awk '{print $1}' || true)"
    [ -n "$needed_bytes" ] || needed_bytes="$(du -sk "$OUT" 2>/dev/null | awk '{print $1 * 1024}' || true)"
    [ -n "$needed_bytes" ] || needed_bytes=0
    out_mount="$(df -P --output=target "$OUT" 2>/dev/null | tail -n1 || true)"
    log ""
    log "[install] generated layout: $(find "$OUT" -type f | wc -l) files, $(human_size "$needed_bytes")"

    # --install-to <mountpoint>: explicit non-interactive target, same
    # eligibility checks as the probe, no menu.
    if [ -n "$INSTALL_TO" ]; then
        INSTALL_TO="${INSTALL_TO%/}"
        [ -z "$INSTALL_TO" ] && INSTALL_TO="/"
        [ "$HOST_WIN" -eq 1 ] && die "--install-to needs a real Linux mountpoint (this is a Windows-host bash)"
        install_targets "$needed_bytes" "$out_mount"
        picked_entry=""
        # ${arr[@]+...} keeps bash 4.2 happy under `set -u` when empty.
        for entry in ${INTERNAL_TARGETS[@]+"${INTERNAL_TARGETS[@]}"} ${USB_TARGETS[@]+"${USB_TARGETS[@]}"}; do
            mp="${entry%%|*}"
            [ "$mp" = "$INSTALL_TO" ] && picked_entry="$entry"
        done
        if [ -z "$picked_entry" ]; then
            die "--install-to $INSTALL_TO is not an eligible target (mounted FAT32 with at least $(human_size "$needed_bytes") free, not ESP//boot, not the --out mount)."
        fi
        IFS='|' read -r mp free size label dev <<< "$picked_entry"
        log "[install] target: $mp (label \"$label\", $(human_size "$free") free)"
        if install_copy "$OUT" "$mp"; then install_boot_hints; fi
        return 0
    fi
    [ "$NO_INSTALL_PROMPT" -eq 1 ] && { log "[install] probe skipped (--no-install-prompt)"; return 0; }

    install_targets "$needed_bytes" "$out_mount"
    if [ "${#INTERNAL_TARGETS[@]}" -eq 0 ] && [ "${#USB_TARGETS[@]}" -eq 0 ]; then
        install_guidance "$needed_bytes"
        return 0
    fi
    if [ ! -t 0 ]; then
        log "[install] no terminal on stdin - skipping the offer (use --install-to)"
        install_guidance "$needed_bytes"
        return 0
    fi

    log "[install] FAT32 install-target probe (nothing already on a target is ever overwritten):"
    picked_entry=""
    for kind in internal usb; do
        if [ "$kind" = "internal" ]; then
            log "[install]   internal, non-boot data partitions (persistent install):"
            entries=(${INTERNAL_TARGETS[@]+"${INTERNAL_TARGETS[@]}"})
        else
            log "[install]   USB / removable media:"
            entries=(${USB_TARGETS[@]+"${USB_TARGETS[@]}"})
        fi
        [ "${#entries[@]}" -eq 0 ] && continue
        n=1
        for entry in "${entries[@]}"; do
            IFS='|' read -r mp free size label dev <<< "$entry"
            log "[install]     [$n] $mp  label=\"$label\"  $(human_size "$free") free of $(human_size "$size")  ($dev)"
            n=$((n + 1))
        done
        ans=""
        read -r ans || ans=""
        case "$ans" in
            ''|*[!0-9]*) continue ;;
        esac
        if [ "$ans" -ge 1 ] && [ "$ans" -le "${#entries[@]}" ]; then
            picked_entry="${entries[$((ans - 1))]}"
            break
        fi
    done
    if [ -z "$picked_entry" ]; then
        install_guidance "$needed_bytes"
        return 0
    fi
    IFS='|' read -r mp free size label dev <<< "$picked_entry"
    log "[install] target: $mp (label \"$label\", $(human_size "$free") free)"
    if install_copy "$OUT" "$mp"; then install_boot_hints; fi
    return 0
}

install_offer
