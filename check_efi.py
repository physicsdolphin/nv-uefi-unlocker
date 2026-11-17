#!/usr/bin/env python3
"""Verify a built EFI artifact's identity before it is flashed or shipped.

Two artifact classes:
- BOOTX64.EFI  (application, PE subsystem 0x0A): classified by banner strings.
- NvUnlockDxe  (DXE driver, PE subsystem 0x0B): classified by the banner
  strings it prints at run time (audit M2: the class must be answerable from
  the artifact itself, not inferred from absence).
"""
import struct
import sys


def subsystem(d: bytes) -> int:
    e = struct.unpack_from("<I", d, 0x3C)[0]
    opt = e + 24
    return struct.unpack_from("<H", d, opt + 68)[0]


path = sys.argv[1] if len(sys.argv) > 1 else "target/x86_64-unknown-uefi/release/loader.efi"
d = open(path, "rb").read()
sub = subsystem(d)
tx = b"TX BUILD: CONTAINS GPU-WRITE PATHS" in d
ro = b"no GPU writes by design" in d  # app banner and driver log line share the tail
print(f"{path}: {len(d)} bytes, PE subsystem 0x{sub:x}")
print(f"  TX banner: {'YES' if tx else 'no'}")
print(f"  read-only banner: {'YES' if ro else 'no'}")
if sub == 0x0B:
    tag = b"nv-uefi-unlocker DXE driver form 1"
    if tag not in d:
        print("  VERDICT: INVALID - DXE form without the build-identity tag (audit F4)")
        sys.exit(1)
    if tx and not ro:
        print("  VERDICT: TX DXE driver build (gated artifact; audit M2 class line)")
        sys.exit(0)
    elif ro and not tx:
        print("  VERDICT: read-only DXE driver build")
        sys.exit(2)
    else:
        print("  VERDICT: UNKNOWN/INVALID - do not flash")
        sys.exit(1)
elif sub == 0x0A:
    if tx and not ro:
        print("  VERDICT: TX build (gated artifact)")
        sys.exit(0)
    elif ro and not tx:
        print("  VERDICT: read-only build")
        sys.exit(2)
    else:
        print("  VERDICT: UNKNOWN/INVALID - do not flash")
        sys.exit(1)
else:
    print("  VERDICT: not an EFI APPLICATION/BOOT_SERVICE_DRIVER image")
    sys.exit(1)
