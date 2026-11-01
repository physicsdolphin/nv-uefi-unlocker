#!/bin/bash
# Build the three QEMU smoke FAT images (run inside WSL).
set -e
cd /mnt/d/git-repo/nvuu-dxe-ffs/release/dxe-smoke
MKFS=/usr/sbin/mkfs.vfat
MCOPY=/usr/sbin/mcopy
MMD=/usr/sbin/mmd
MDIR=/usr/sbin/mdir

for img in fat-armed fat-noarm fat-app; do
  rm -f "$img.img"
  dd if=/dev/zero of="$img.img" bs=1M count=64 status=none
  "$MKFS" -F 32 "$img.img" >/dev/null
done

printf 'load fs0:\\NvUnlockDxe-readonly.efi\r\n' > s.nsh
"$MCOPY" -i fat-armed.img s.nsh ::startup.nsh
"$MCOPY" -i fat-armed.img NvUnlockDxe-readonly.efi ::
printf '0\n' > arm.txt
"$MCOPY" -i fat-armed.img arm.txt ::tx-dxe-arm.txt

"$MCOPY" -i fat-noarm.img s.nsh ::startup.nsh
"$MCOPY" -i fat-noarm.img NvUnlockDxe-readonly.efi ::

"$MMD"   -i fat-app.img ::EFI
"$MMD"   -i fat-app.img ::EFI/BOOT
"$MCOPY" -i fat-app.img ../../target/x86_64-unknown-uefi/release/loader.efi ::EFI/BOOT/BOOTX64.EFI

echo IMAGES_OK
echo '--- fat-armed:'
"$MDIR" -i fat-armed.img ::
echo '--- fat-app:'
"$MDIR" -i fat-app.img ::/EFI/BOOT
