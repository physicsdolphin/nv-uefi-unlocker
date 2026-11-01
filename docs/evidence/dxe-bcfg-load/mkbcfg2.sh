#!/bin/bash
# BCFG stage-2 image fix: image B must carry the EXACT control-file set that
# was fingerprinted at install time (arm-install marker included), and gets
# a self-erasing bootstrap that clears duplicate Driver#### entries (the
# reset-in-startup.nsh loop of the first attempt added one per boot), adds
# exactly one, and removes itself before reset.
set -e
cd /root/dxe-smoke
MCOPY=/usr/sbin/mcopy
MDIR=/usr/sbin/mdir
ROOT=/mnt/d/git-repo/nvuu-dxe-ffs

export MTOOLSRC=/root/dxe-smoke/mtoolsrc
/usr/sbin/mformat b:

cat > s.nsh <<'NSEOF'
@echo -off
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver rm 0
bcfg driver add 0 fs0:\NvUnlockDxe-readonly.efi "NvUnlockDxe"
bcfg driver dump
rm fs0:\startup.nsh
reset
NSEOF
$MCOPY s.nsh b:/startup.nsh
$MCOPY $ROOT/release/dxe-smoke/NvUnlockDxe-readonly.efi b:
printf '7\n' > arm.txt
$MCOPY arm.txt b:/tx-dxe-arm.txt
: > noconfirm.txt
$MCOPY noconfirm.txt b:/tx-dxe-noconfirm.txt
printf '1\n' > arm-install.txt
$MCOPY arm-install.txt b:/tx-dxe-arm-install.txt

cp bcfg-b.img $ROOT/release/dxe-smoke/
echo B2_OK
$MDIR b:/ | head -8
