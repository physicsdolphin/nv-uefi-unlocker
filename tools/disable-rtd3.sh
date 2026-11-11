#!/usr/bin/env bash
# disable-rtd3.sh - one-shot fixer for "NVIDIA runtime D3 enabled" on Linux.
#
# The volatile unlock is wiped when runtime power management parks the dGPU
# in D3cold after handoff (issue #9). The documented fix (docs/faq.md#linux)
# is NVreg_DynamicPowerManagement=0x00 in modprobe.d; this script applies the
# whole documented procedure in one command:
#
#   sudo ./tools/disable-rtd3.sh          apply (persistent conf + live keep-on)
#   sudo ./tools/disable-rtd3.sh --check  read-only status report, no writes
#
# What it writes: exactly one file, /etc/modprobe.d/nvidia-runtimepm.conf
# (same-named override of a distro-shipped /usr/lib/modprobe.d copy, per the
# FAQ; an existing /etc copy is backed up first). It never edits or deletes
# other files, never unloads modules, never reboots. It also sets
# power/control=on on the dGPU(s) for an immediate (non-persistent) effect
# until the next boot, when the module option above takes over.
#
# SPDX-License-Identifier: GPL-2.0-only

set -u

TARGET_CONF="/etc/modprobe.d/nvidia-runtimepm.conf"
OPT_LINE="options nvidia NVreg_DynamicPowerManagement=0x00"
CONF_DIRS="/run/modprobe.d /etc/modprobe.d /usr/local/lib/modprobe.d /usr/lib/modprobe.d"

say()  { printf '%s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*"; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

# --- discovery ---------------------------------------------------------------

nvidia_dgpus() {
    local d vend cls
    for d in /sys/bus/pci/devices/*; do
        [ -r "$d/vendor" ] || continue
        vend=$(cat "$d/vendor" 2>/dev/null)
        cls=$(cat "$d/class" 2>/dev/null)
        [ "$vend" = "0x10de" ] || continue
        case "$cls" in
            0x0300*|0x0302*) printf '%s\n' "${d##*/}" ;;
        esac
    done
}

driver_param() {
    # Current live value of DynamicPowerManagement (empty if driver absent).
    [ -r /proc/driver/nvidia/params ] || return 0
    grep -E '^DynamicPowerManagement:' /proc/driver/nvidia/params | head -n1
}

merged_modprobe() {
    # The kmod-merged truth: every DynamicPowerManagement assignment in effect.
    have modprobe || return 0
    modprobe -c 2>/dev/null | grep -i 'DynamicPowerManagement' || true
}

conf_files_mentioning_dpm() {
    # Every modprobe.d file that sets DynamicPowerManagement, with its value.
    local dir f val
    for dir in $CONF_DIRS; do
        [ -d "$dir" ] || continue
        for f in "$dir"/*; do
            [ -f "$f" ] || continue
            case "$f" in *.conf) ;; *) continue ;; esac
            val=$(grep -h 'NVreg_DynamicPowerManagement' "$f" 2>/dev/null | head -n1)
            [ -n "$val" ] && printf '%s: %s\n' "$f" "$val"
        done
    done
}

status() {
    local dev rc
    say "== NVIDIA dGPUs =="
    local found=0
    for dev in $(nvidia_dgpus); do
        found=1
        rc=""
        [ -r "/sys/bus/pci/devices/$dev/power/control" ] && rc=$(cat "/sys/bus/pci/devices/$dev/power/control" 2>/dev/null)
        local rst pst
        rst=$(cat "/sys/bus/pci/devices/$dev/power/runtime_status" 2>/dev/null)
        pst=$(cat "/sys/bus/pci/devices/$dev/power_state" 2>/dev/null)
        say "  $dev  power/control=$rc  runtime_status=$rst  power_state=$pst"
    done
    [ "$found" = "1" ] || say "  (none found)"
    say "== driver param =="
    local p
    p=$(driver_param)
    [ -n "$p" ] && say "  $p" || say "  (nvidia driver not loaded - /proc/driver/nvidia/params absent)"
    say "== modprobe.d files setting DynamicPowerManagement =="
    local files
    files=$(conf_files_mentioning_dpm)
    [ -n "$files" ] && printf '%s\n' "$files" | sed 's/^/  /' || say "  (none)"
    say "== merged config (modprobe -c) =="
    local m
    m=$(merged_modprobe)
    [ -n "$m" ] && printf '%s\n' "$m" | sed 's/^/  /' || say "  (no DynamicPowerManagement assignment in effect)"
}

verdict() {
    local p mtxt dev bad_pc=0
    p=$(driver_param)
    mtxt=$(merged_modprobe)
    # Not fixed if: driver visible with a nonzero DPM, or merged config lacks a 0x00 assignment.
    if [ -n "$p" ] && ! printf '%s\n' "$p" | grep -qiE 'DynamicPowerManagement:[[:space:]]*(0x0+|0)([[:space:]]|$)'; then
        say "STATE: NOT FIXED - driver still runs runtime D3 ($p)"
    elif printf '%s\n' "$mtxt" | grep -qiE 'DynamicPowerManagement=(0x0+|0)([[:space:]]|$)'; then
        say "STATE: fixed in config (merged config pins DynamicPowerManagement=0)"
    else
        say "STATE: not applied yet (no 0x00 assignment in merged config)"
    fi
    for dev in $(nvidia_dgpus); do
        [ -r "/sys/bus/pci/devices/$dev/power/control" ] || continue
        [ "$(cat "/sys/bus/pci/devices/$dev/power/control" 2>/dev/null)" = "on" ] || bad_pc=1
    done
    [ "$bad_pc" = "0" ] && say "live power/control: on (no runtime park)" || say "live power/control: not forced to on"
}

# --- apply -------------------------------------------------------------------

apply_live_keepon() {
    local dev
    for dev in $(nvidia_dgpus); do
        if [ -w "/sys/bus/pci/devices/$dev/power/control" ]; then
            echo on > "/sys/bus/pci/devices/$dev/power/control" 2>/dev/null \
                && say "live: $dev power/control -> on (immediate, until reboot)" \
                || warn "could not set power/control on $dev"
        fi
    done
}

apply_conf() {
    local usrlib="/usr/lib/modprobe.d/nvidia-runtimepm.conf"
    if [ -e "$TARGET_CONF" ]; then
        local bak="$TARGET_CONF.bak.$(date +%Y%m%d%H%M%S)"
        cp -p "$TARGET_CONF" "$bak" && say "backup: $TARGET_CONF -> $bak"
    fi
    if [ -f "$usrlib" ]; then
        say "distro ships $usrlib - writing the same-named /etc override (kmod precedence)"
    else
        say "no distro $usrlib found - writing /etc/modprobe.d/nvidia-runtimepm.conf"
    fi
    printf '%s\n' "$OPT_LINE" > "$TARGET_CONF" \
        && say "written: $TARGET_CONF <- '$OPT_LINE'" \
        || die "could not write $TARGET_CONF"
}

warn_conflicts() {
    # Differently-named files setting the same option accumulate in kmod; the
    # merged dump is the truth. Flag anything that still asks for nonzero DPM.
    local merged
    merged=$(merged_modprobe | grep -i 'NVreg_DynamicPowerManagement' | grep -ivE '=(0x0+|0)$' || true)
    if [ -n "$merged" ]; then
        warn "a nonzero DynamicPowerManagement is still present in the merged config:"
        printf '%s\n' "$merged" | sed 's/^/  /' >&2
        warn "edit or comment out the source file listed above (this script never edits other files)"
    fi
}

rebuild_initramfs() {
    if have update-initramfs; then
        say "initramfs: running update-initramfs -u (Debian/Ubuntu)"
        update-initramfs -u || warn "update-initramfs failed - run it manually before rebooting"
    elif have dracut; then
        say "initramfs: running dracut -f (Fedora/RHEL)"
        dracut -f || warn "dracut failed - run it manually before rebooting"
    elif have mkinitcpio; then
        say "initramfs: Arch detected - modprobe.d is read at module load; if nvidia loads from the initramfs, run: sudo mkinitcpio -P"
    else
        say "initramfs: no known tool found - if nvidia loads from your initramfs, rebuild it manually"
    fi
}

apply() {
    say "== applying the docs/faq.md#linux RTD3 fix =="
    apply_conf
    apply_live_keepon
    warn_conflicts
    rebuild_initramfs
    say
    say "== result =="
    status
    verdict
    say
    say "Next steps:"
    say "  1. reboot (the module option applies at nvidia load),"
    say "     then re-check with: sudo $0 --check"
    say "  2. if a D3cold park already wiped the unlock, re-run the USB stick"
    say "     transaction - rewriting this config does not restore it (issue #9)"
    say "  3. verify in-OS: sudo python3 tools/diag-unlock.py   (SS override live)"
}

# --- entry -------------------------------------------------------------------

case "${1:-}" in
    --check|-c)
        status
        verdict
        exit 0
        ;;
    --help|-h)
        say "usage: sudo $0 [--check]"
        say "  (default) apply the FAQ#linux RTD3 fix: modprobe override + live keep-on + initramfs rebuild"
        say "  --check   read-only status report"
        exit 0
        ;;
    "")
        [ "$(id -u)" = "0" ] || die "need root (sudo) to write /etc/modprobe.d and power/control; try: sudo $0"
        apply
        ;;
    *)
        die "unknown argument: $1 (try --help)"
        ;;
esac
