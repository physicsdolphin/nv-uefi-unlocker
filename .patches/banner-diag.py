# banner.rs three-way + check_efi.py diag verdict.
import io

p = r'loader/src/banner.rs'
s = io.open(p, encoding='utf-8').read()
old = r'''    #[cfg(not(feature = "tx"))]
    {
        info!(" read-only build: enumerate + sample + chain-load");
        info!(" console output only; no GPU writes by design");
    }'''
new = r'''    #[cfg(all(feature = "diag", not(feature = "tx")))]
    {
        info!(" ** DIAG BUILD: SURGERY ONLY - NO GPU-WRITE PATHS COMPILED **");
        info!(" \\tx-agitate-only.txt = full-surgery zero-GPU-write control;");
        info!(" \\tx-ebs-watch.txt = ExitBootServices observer; transaction code is not compiled in");
    }
    #[cfg(not(any(feature = "tx", feature = "diag")))]
    {
        info!(" read-only build: enumerate + sample + chain-load");
        info!(" console output only; no GPU writes by design");
    }'''
assert s.count(old) == 1, "banner block"
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='').write(s)
print("banner patched")

p = r'check_efi.py'
s = io.open(p, encoding='utf-8').read()
old = r'''tx = b"TX BUILD: CONTAINS GPU-WRITE PATHS" in d
ro = b"console output only; no GPU writes by design" in d
print(f"{path}: {len(d)} bytes")
print(f"  TX banner: {'YES' if tx else 'no'}")
print(f"  read-only banner: {'YES' if ro else 'no'}")
if tx and not ro:
    print("  VERDICT: TX build (gated artifact)")
    sys.exit(0)
elif ro and not tx:
    print("  VERDICT: read-only build")
    sys.exit(2)
else:
    print("  VERDICT: UNKNOWN/INVALID - do not flash")
    sys.exit(1)'''
new = r'''tx = b"TX BUILD: CONTAINS GPU-WRITE PATHS" in d
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
    sys.exit(1)'''
assert s.count(old) == 1, "check_efi block"
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='').write(s)
print("check_efi patched")
