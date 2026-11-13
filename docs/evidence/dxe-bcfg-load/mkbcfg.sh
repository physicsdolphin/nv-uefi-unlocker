#!/bin/bash
# BCFG smoke images (WSL-native FS; see mkimages3.sh for the pattern).
# A = installer app (BOOTX64) + all markers + driver
# B = shell stage (startup.nsh with bcfg add, NO \EFI\BOOT) - reused for
#     the armed run and the no-variable negative control
set -e
cd /root/dxe-smoke
MCOPY=/usr/sbin/mcopy
MMD=/usr/sbin/mmd
MDIR=/usr/sbin/mdir
ROOT=/mnt/d/git-repo/nvuu-dxe-ffs

python3 - <<'PYEOF'
import struct
mbr = bytearray(512)
e = bytearray(16)
e[4] = 0x0C
struct.pack_into("<I", e, 8, 2048)
struct.pack_into("<I", e, 12, 131072)
mbr[446:462] = e
mbr[510:512] = b"\x55\xaa"
for name in ("bcfg-a", "bcfg-b"):
    with open(name + ".img", "wb") as f:
        f.write(mbr)
        f.truncate(68 * 1024 * 1024)
PYEOF

cat > mtoolsrc <<'MTEOF'
drive a: file="/root/dxe-smoke/bcfg-a.img" partition=1
drive b: file="/root/dxe-smoke/bcfg-b.img" partition=1
MTEOF
export MTOOLSRC=/root/dxe-smoke/mtoolsrc

/usr/sbin/mformat a:
/usr/sbin/mformat b:

# Image A: installer app + driver + markers (tx-OFF: manifest not required).
$MMD a:/EFI
$MMD a:/EFI/BOOT
$MCOPY $ROOT/target/x86_64-unknown-uefi/release/loader.efi a:/EFI/BOOT/BOOTX64.EFI
$MCOPY $ROOT/release/dxe-smoke/NvUnlockDxe-readonly.efi a:
printf '1\n' > arm-install.txt
$MCOPY arm-install.txt a:/tx-dxe-arm-install.txt
printf '7\n' > arm.txt
$MCOPY arm.txt a:/tx-dxe-arm.txt
: > noconfirm.txt
$MCOPY noconfirm.txt a:/tx-dxe-noconfirm.txt

# Image B: shell stage - startup.nsh adds the Driver#### option, then resets.
printf 'load fs0:\\NvUnlockDxe-readonly.efi\r\n' > s-load.nsh
cat > s.nsh <<'NSEOF'
@echo -off
bcfg driver dump
bcfg driver add 0 fs0:\NvUnlockDxe-readonly.efi "NvUnlockDxe"
bcfg driver dump
reset
NSEOF
$MCOPY s.nsh b:/startup.nsh
$MCOPY $ROOT/release/dxe-smoke/NvUnlockDxe-readonly.efi b:
$MCOPY arm.txt b:/tx-dxe-arm.txt
$MCOPY noconfirm.txt b:/tx-dxe-noconfirm.txt

cp bcfg-a.img bcfg-b.img $ROOT/release/dxe-smoke/
echo BCFG_IMAGES_OK
export MTOOLSRC=/root/dxe-smoke/mtoolsrc
echo "--- A:"; $MDIR a:/ | head -8
echo "--- B:"; $MDIR b:/ | head -6
