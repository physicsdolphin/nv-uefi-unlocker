#!/usr/bin/env python3
"""Verify loader.efi is the TX build (banner marker present)."""
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "target/x86_64-unknown-uefi/release/loader.efi"
d = open(path, "rb").read()
tx = b"TX BUILD: CONTAINS GPU-WRITE PATHS" in d
ro = b"console output only; no GPU writes by design" in d
diag = b"DIAG BUILD: SURGERY ONLY" in d
print(f"{path}: {len(d)} bytes")
print(f"  TX banner: {'YES' if tx else 'no'}")
print(f"  read-only banner: {'YES' if ro else 'no'}")
print(f"  diag banner: {'YES' if diag else 'no'}")
if tx and not ro and not diag:
    print("  VERDICT: TX build (gated artifact)")
    sys.exit(0)
elif ro and not tx and not diag:
    print("  VERDICT: read-only build")
    sys.exit(2)
elif diag and not tx and not ro:
    print("  VERDICT: diag build (surgery only, zero-write control)")
    sys.exit(3)
else:
    print("  VERDICT: UNKNOWN/INVALID - do not flash")
    sys.exit(1)
