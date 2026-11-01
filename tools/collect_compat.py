#!/usr/bin/env python3
"""collect_compat.py - normalize raw evidence into a compatibility profile.

P2-23 of docs/peer-landscape-2026-11.md (impacifist-style offline
collection, minus the profile generation): point it at a loader.log
and/or ssdiag output and it emits one JSON with the fields the
compatibility matrix triages on - device ID, family, the FEAT/selector/
WPR2 verdicts, XP3G line, and the loader's RESULT/FINAL/HANDOFF verdict
lines verbatim. Nothing here runs the tool or touches hardware; it only
parses text you already have.

Usage:
  python tools/collect_compat.py --log loader.log --log ssdiag.txt \
      --bdf 01:00.0 --platform "B760 + 12700K" --note "first 2709 report" \
      --out profile.json

Output schema nvuu-compat-profile/1; sources carry per-file SHA-256 so a
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
# ssdiag-style lines: "<NAME> ... 0x<value>" or "<NAME>: 0x<value>"
KV_RE = re.compile(r"^\s*([A-Za-z][A-Za-z0-9_ /.:-]{2,40}?)[\s:]+(0x[0-9a-fA-F]{1,8})\b")
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


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def collect(path: str, fields: dict, verdicts: list, devids: set) -> None:
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            for m in DEVICE_RE.finditer(line):
                devids.add(int(m.group(1), 16))
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
    ap.add_argument("--platform", default="", help="board/CPU/OS/driver one-liner")
    ap.add_argument("--note", default="", help="free-form context")
    ap.add_argument("--out", default="compat-profile.json")
    args = ap.parse_args()

    fields: dict = {}
    verdicts: list = []
    devids: set = set()
    sources = []
    for path in args.log:
        collect(path, fields, verdicts, devids)
        sources.append({"file": path, "sha256": sha256_file(path)})

    gpus = [{
        "device_id": f"10de:{d:04x}",
        "family": family_of(d),
        "bdf": args.bdf if len(devids) == 1 else "",
        "fields": fields,
    } for d in sorted(devids)] or [{"device_id": "unknown", "family": "", "bdf": args.bdf, "fields": fields}]

    profile = {
        "schema": "nvuu-compat-profile/1",
        "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "sources": sources,
        "platform": args.platform,
        "gpus": gpus,
        "loader_verdicts": verdicts[:40],
        "note": args.note,
    }
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(profile, f, ensure_ascii=False, indent=2)
    print(f"profile: {args.out}  gpus={len(gpus)} fields={len(fields)} verdicts={len(verdicts)}")
    for g in gpus:
        print(f"  {g['device_id']}  {g['family']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
