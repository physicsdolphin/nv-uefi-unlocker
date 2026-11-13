#!/bin/bash
# Partitioned smoke images, built on the WSL-native FS (9p is too slow for mformat).
set -e
cd /root/dxe-smoke
MCOPY=/usr/sbin/mcopy
MMD=/usr/sbin/mmd
MDIR=/usr/sbin/mdir

python3 - <<'PYEOF'
import struct
mbr = bytearray(512)
e = bytearray(16)
e[4] = 0x0C
struct.pack_into("<I", e, 8, 2048)
struct.pack_into("<I", e, 12, 131072)
mbr[446:462] = e
mbr[510:512] = b"\x55\xaa"
for name in ("fat-install", "fat-armed", "fat-noarm", "fat-appreg"):
    with open(name + ".img", "wb") as f:
        f.write(mbr)
        f.truncate(68 * 1024 * 1024)
PYEOF

cat > mtoolsrc <<'MTEOF'
drive i: file="/root/dxe-smoke/fat-install.img" partition=1
drive a: file="/root/dxe-smoke/fat-armed.img" partition=1
drive n: file="/root/dxe-smoke/fat-noarm.img" partition=1
drive r: file="/root/dxe-smoke/fat-appreg.img" partition=1
MTEOF
export MTOOLSRC=/root/dxe-smoke/mtoolsrc

for d in i: a: n: r:; do /usr/sbin/mformat "$d"; done

# Run A - installer app with \tx-dxe-arm-install.txt.
$MMD i:/EFI
$MMD i:/EFI/BOOT
$MCOPY /mnt/d/git-repo/nvuu-dxe-ffs/target/x86_64-unknown-uefi/release/loader.efi i:/EFI/BOOT/BOOTX64.EFI
printf '1\n' > arm-install.txt
$MCOPY arm-install.txt i:/tx-dxe-arm-install.txt

# Run B - armed driver, arm marker 7s.
printf 'load fs0:\\NvUnlockDxe-readonly.efi\r\n' > s.nsh
$MCOPY s.nsh a:/startup.nsh
$MCOPY /mnt/d/git-repo/nvuu-dxe-ffs/release/dxe-smoke/NvUnlockDxe-readonly.efi a:/
printf '7\n' > arm.txt
$MCOPY arm.txt a:/tx-dxe-arm.txt

# Run C - driver, marker present but NO NVRAM authority.
$MCOPY s.nsh n:/startup.nsh
$MCOPY /mnt/d/git-repo/nvuu-dxe-ffs/release/dxe-smoke/NvUnlockDxe-readonly.efi n:/
$MCOPY arm.txt n:/tx-dxe-arm.txt

# Run D - app regression (no install marker).
$MMD r:/EFI
$MMD r:/EFI/BOOT
$MCOPY /mnt/d/git-repo/nvuu-dxe-ffs/target/x86_64-unknown-uefi/release/loader.efi r:/EFI/BOOT/BOOTX64.EFI

cp fat-install.img fat-armed.img fat-noarm.img fat-appreg.img /mnt/d/git-repo/nvuu-dxe-ffs/release/dxe-smoke/
echo IMAGES_OK
for d in i: a: n: r:; do echo "--- $d"; $MDIR "$d":: | head -5; done
