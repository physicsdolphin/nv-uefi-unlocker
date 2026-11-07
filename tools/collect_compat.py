#!/usr/bin/env python3
"""collect_compat.py - normalize raw evidence into a compatibility profile.

P2-23 of docs/peer-landscape-2026-11.md (impacifist-style offline
collection, minus the profile generation): point it at a loader.log
and/or ssdiag output and it emits one JSON with the fields the
compatibility matrix triages on - device ID, family, the FEAT/selector/
WPR2 verdicts, XP3G line, and the loader's RESULT/FINAL/HANDOFF verdict
lines verbatim. Since ssdiag 0.7.0 the same logs carry an inventory
block (`ssdiag --inventory`, also printed by every full run and by
diag-unlock.py --inventory), which is parsed here into the platform
object and the per-GPU desc/subsys/BDF/VBIOS/driver fields - so the
old manual --platform one-liner is now only an override.
Nothing here runs the tool or touches hardware; it only
parses text you already have.

Usage:
  python tools/collect_compat.py --log loader.log --log ssdiag.txt \
      --bdf 01:00.0 --platform "B760 + 12700K" --note "first 2709 report" \
      --out profile.json

Output schema nvuu-compat-profile/2; sources carry per-file SHA-256 so a
profile can be checked against the attached logs later.
"""

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone

# loader.log SAMPLE lines: "SAMPLE <name> = 0x<value> [bucket]"
SAMPLE_RE = re.compile(r"SAMPLE\s+(.+?)\s*=\s*(0x[0-9a-fA-F]+)(?:\s*\[(\w+)\])?")
# ssdiag-style lines: "<NAME> ... 0x<value>", "<NAME>: 0x<value>" or the
# core/debug block's "<NAME> = 0x<value>" (the '=' separator is what real
# ssdiag dumps print - without it in the class the whole block parsed as
# zero fields)
KV_RE = re.compile(r"^\s*([A-Za-z][A-Za-z0-9_ /.:-]{2,40}?)[\s:=]+(0x[0-9a-fA-F]{1,8})\b")
# canonical fields worth surfacing (case-insensitive contains-match)
CANONICAL = [
    "PMC_BOOT_0", "FEAT", "SS0", "SS1", "READOUT", "WPR2", "GSP_CPUCTL",
    "SEC2_CPUCTL", "XP3G", "PSTRAPS", "CYA", "CFG", "BAR1_CAP", "LNK",
    "FB_SIZE", "VGA_WORKSPACE", "GFW",
]
VERDICT_RE = re.compile(
    r"(RESULT:|FINAL:|HANDOFF[- ].*:|claim: (EALREADY|EBUSY|.*accept)|"
    r"POST-FLR:|POST-BOOT:|booter returned|unlock landed|selected .* GPU)")
DEVICE_RE = re.compile(r"10de:([0-9a-fA-F]{4})")

# ---- ssdiag 0.7.0 / diag-unlock inventory block (stable KEY: prefixes) ----
# platform lines: "OS:      Windows 11 24H2 build 26100.2894" etc.
INV_PLATFORM_RES = {
    "os": re.compile(r"^OS:\s+(.+)$"),
    "cpu": re.compile(r"^CPU:\s+(.+)$"),
    "board": re.compile(r"^BOARD:\s+(.+)$"),
    "bios": re.compile(r"^BIOS:\s+(.+)$"),
    "chassis": re.compile(r"^CHASSIS:\s+(.+)$"),
    "ram": re.compile(r"^RAM:\s+(.+)$"),
}
# ssdiag 0.7.1 list rows: many entries per machine, stored as lists
INV_LIST_RE = re.compile(r"^(RAM SLOT|DISK|PART|PCI DEV):\s+(.+)$")
INV_LIST_KEYS = {
    "RAM SLOT": "ram_slots",
    "DISK": "disks",
    "PART": "partitions",
    "PCI DEV": "pci_devices",
}
# per-GPU lines; the GPU index n ties them together within one block
INV_GPU_RE = re.compile(r"^GPU (\d+): (.+)$")
INV_PCI_RE = re.compile(
    r"^GPU (\d+) PCI:\s+10de:([0-9a-fA-F]{4})"
    r"(?:\s+SUBSYS ([0-9A-Fa-f]{8}))?"
    r"(?:\s+BDF ([0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}\.[0-7]))?")
INV_FAMILY_RE = re.compile(
    r"^GPU (\d+) family:\s+(.+?)\s+-\s+(IN-SCOPE|OUT-OF-SCOPE)")
INV_VBIOS_RE = re.compile(r"^GPU (\d+) VBIOS:\s+(\S+?)(?:\s+\(nvapi\))?$")
INV_DRIVER_RE = re.compile(r"^GPU (\d+) driver:\s+([0-9.]+)(?:\s+\(([^)]+)\))?")
# Loader enumeration lines carry the PCI class code:
#   "PCI 0000:01:00.0 10de:1f07 class 000300"  (000300 = display controller)
# NVIDIA GPUs are multifunction: fn0 = display (class 0300), fn1 = HDA audio
# (class 0403), fn2/3 = USB-C/USB4 (class 0c03/0c80). The class code is the
# authoritative display-function discriminator (function numbers are
# convention, not spec). Lines without a class fall back to DEVICE_RE with
# the non-display known-ID exclusion below.
PCI_ENUM_RE = re.compile(
    r"PCI\s+([0-9a-fA-F]{4}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-7])"
    r"\s+10de:([0-9a-fA-F]{4})\s+class\s+([0-9a-fA-F]{6})")
DISPLAY_CLASSES = {"000300", "000301", "000302"}
# NVIDIA non-display function IDs seen in the field (audio fn / USB-C fn):
# they share the die with a real GPU and must not become GPU rows.
NON_DISPLAY_IDS = {"10f9", "10fa", "1ada", "1adb", "22ba", "22bb", "22be", "22bf",
                   "22e8", "22e9", "22ea", "22eb", "22ec", "22ed"}

FAMILY_RANGES = [
    ((0x2600, 0x28FF), "ad102 (Ada)"),
    ((0x2200, 0x223F), "ga102 (Ampere)"),
    ((0x2400, 0x25FF), "ga102 (Ampere, GA103/104/106/107 segment)"),
    ((0x1E02, 0x1FFF), "tu10x (Turing)"),
    ((0x2182, 0x21D1), "tu10x (Turing TU116/117)"),
]


def family_of(devid: int) -> str:
    for (lo, hi), name in FAMILY_RANGES:
        if lo <= devid <= hi:
            return name
    return "outside the three transaction families"


def collect_inventory(path: str, platform: dict, gpus: dict) -> None:
    """Parse the ssdiag 0.7.0 / diag-unlock inventory block out of one log.

    `gpus` maps device-id -> {desc, subsys, bdf, family, verdict, vbios,
    driver}; a second file carrying another block only fills keys still
    missing (setdefault everywhere)."""
    with open_log(path) as f:
        idx = {}  # GPU index n -> device id (from the PCI line)
        for line in f:
            line = line.rstrip("\n")
            for key, rex in INV_PLATFORM_RES.items():
                m = rex.match(line)
                if m and key not in platform:
                    platform[key] = m.group(1).strip()
            lm = INV_LIST_RE.match(line)
            if lm:
                platform.setdefault(INV_LIST_KEYS[lm.group(1)], []).append(lm.group(2).strip())
                continue
            m = INV_GPU_RE.match(line)
            if m:
                idx_key = int(m.group(1))
                if idx_key not in idx:
                    idx[idx_key] = None  # placeholder until its PCI line
                    descs = gpus.setdefault("__descs__", {})
                    descs.setdefault(str(idx_key), m.group(2).strip())
                continue
            m = INV_PCI_RE.match(line)
            if m:
                n, devid = int(m.group(1)), m.group(2).lower()
                idx[n] = devid
                g = gpus.setdefault(devid, {})
                descs = gpus.get("__descs__", {})
                if descs.get(str(n)):
                    g.setdefault("desc", descs[str(n)])
                for k, v in (("subsys", m.group(3)), ("bdf", m.group(4))):
                    if v and not g.get(k):
                        g[k] = v.lower() if k == "bdf" else v.upper()
                continue
            m = INV_FAMILY_RE.match(line)
            if m:
                devid = idx.get(int(m.group(1)))
                if devid:
                    g = gpus.setdefault(devid, {})
                    g.setdefault("family", m.group(2).strip())
                    g.setdefault("verdict", m.group(3))
                continue
            m = INV_VBIOS_RE.match(line)
            if m and m.group(2) != "unknown":
                devid = idx.get(int(m.group(1)))
                if devid:
                    gpus.setdefault(devid, {}).setdefault("vbios", m.group(2))
                continue
            m = INV_DRIVER_RE.match(line)
            if m:
                devid = idx.get(int(m.group(1)))
                if devid:
                    gpus.setdefault(devid, {}).setdefault(
                        "driver", m.group(3) or m.group(2))
                continue


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def open_log(path: str):
    """UTF-16-aware open: PowerShell 5.1 '>' redirects land here as
    UTF-16LE with a BOM, and every regex below would silently match
    nothing against NUL-interleaved text."""
    with open(path, "rb") as f:
        head = f.read(2)
    enc = "utf-16" if head in (b"\xff\xfe", b"\xfe\xff") else "utf-8"
    return open(path, "r", encoding=enc, errors="replace")


def collect(path: str, fields: dict, verdicts: list, devids: set) -> None:
    with open_log(path) as f:
        for line in f:
            em = PCI_ENUM_RE.search(line)
            if em:
                # authoritative: enumeration line with class code
                if em.group(3) in DISPLAY_CLASSES:
                    devids.add(int(em.group(2), 16))
                continue  # non-display functions (audio/USB-C) never become GPUs
            # devid fallback for logs without enum-class lines: only trust
            # lines that explicitly reference a GPU (SAMPLE target / selection
            # / per-card headers) - bare 10de: mentions (assets, URLs) never
            # create GPU rows, and known non-display IDs are excluded. The
            # keyword gate guards ONLY this block: gating the whole loop (the
            # pre-0.7 shape) silently dropped every SAMPLE/KV value line of
            # real loader.log/ssdiag dumps - fields parsed as 0.
            if re.search(r"GPU |selected |target |FOUND", line, re.I):
                for m in DEVICE_RE.finditer(line):
                    did = int(m.group(1), 16)
                    if f"{did:04x}" in NON_DISPLAY_IDS:
                        continue  # known audio/USB-C function IDs
                    devids.add(did)
                # ssdiag per-card headers carry "PCI\VEN_10DE&DEV_1F07" - the
                # colon form does not appear there, so without this branch a
                # pure-ssdiag log contributed no device id at all
                for m in re.finditer(r"DEV_([0-9A-Fa-f]{4})", line):
                    did = int(m.group(1), 16)
                    if f"{did:04x}" in NON_DISPLAY_IDS:
                        continue
                    devids.add(did)
            sm = SAMPLE_RE.search(line)
            if sm:
                name, val = sm.group(1).strip(), int(sm.group(2), 16)
                bucket = sm.group(3) or ""
                for key in CANONICAL:
                    if key.lower() in name.lower():
                        fields[name] = {"value": val, "bucket": bucket}
                        break
                continue
            vm = VERDICT_RE.search(line)
            if vm:
                verdicts.append(line.strip()[:300])
                continue
            km = KV_RE.match(line)
            if km:
                name, val = km.group(1).strip(), int(km.group(2), 16)
                for key in CANONICAL:
                    if key.lower() in name.lower():
                        fields.setdefault(name, {"value": val, "bucket": ""})
                        break


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--log", action="append", required=True,
                    help="loader.log and/or ssdiag output (repeatable)")
    ap.add_argument("--bdf", default="", help="PCI address of the target card")
    ap.add_argument("--platform", default="",
                    help="manual platform note; the ssdiag/diag-unlock "
                         "inventory block (when present) fills the "
                         "structured platform object instead")
    ap.add_argument("--note", default="", help="free-form context")
    ap.add_argument("--out", default="compat-profile.json")
    args = ap.parse_args()

    fields: dict = {}
    verdicts: list = []
    devids: set = set()
    inv_platform: dict = {}
    inv_gpus: dict = {}
    sources = []
    for path in args.log:
        collect(path, fields, verdicts, devids)
        collect_inventory(path, inv_platform, inv_gpus)
        sources.append({"file": path, "sha256": sha256_file(path)})
    inv_gpus.pop("__descs__", None)
    devids |= {int(d, 16) for d in inv_gpus}

    gpus = []
    for d in sorted(devids):
        did = f"{d:04x}"
        inv = inv_gpus.get(did, {})
        gpus.append({
            "device_id": f"10de:{did}",
            "family": inv.get("family", family_of(d)),
            "family_verdict": inv.get("verdict", ""),
            "desc": inv.get("desc", ""),
            "subsys": inv.get("subsys", ""),
            "vbios": inv.get("vbios", ""),
            "driver": inv.get("driver", ""),
            "bdf": inv.get("bdf") or (args.bdf if len(devids) == 1 else ""),
            "fields": fields,
        })
    if not gpus:
        gpus = [{"device_id": "unknown", "family": "", "family_verdict": "",
                 "desc": "", "subsys": "", "vbios": "", "driver": "",
                 "bdf": args.bdf, "fields": fields}]

    profile = {
        "schema": "nvuu-compat-profile/2",
        "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "sources": sources,
        "platform": inv_platform,
        "platform_note": args.platform,
        "gpus": gpus,
        "loader_verdicts": verdicts[:40],
        "note": args.note,
    }
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(profile, f, ensure_ascii=False, indent=2)
    print(f"profile: {args.out}  gpus={len(gpus)} fields={len(fields)} "
          f"verdicts={len(verdicts)} platform_keys={len(inv_platform)}")
    for g in gpus:
        print(f"  {g['device_id']}  {g['family']}"
              + (f"  {g['vbios']}" if g["vbios"] else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
