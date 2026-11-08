#!/usr/bin/env python3
"""Carve loader.log text out of a raw FAT32 volume image (issue #170/#122
recipe: dirent says 0 bytes, flushed lines live in clusters). Scans every
cluster for loader-log line patterns and stitches contiguous runs into
candidate sessions.

usage: python -I carve-log.py <image> <out-dir>
"""
import os
import re
import sys

LINE = re.compile(rb"(BOOT: logger|WATCHDOG|BANNER|SELECTOR|NATIVE-GOP|TX-|TX:|DETACH|POST-FLR|POST-GOP|HANDOFF|FINAL|CHAIN|loader |GOP|QUIET|FLUSH|IDENTITY|CONSOLE|EFI |SFS|volume)")
MIN_RUN = 2  # consecutive pattern hits to count as a session region


def main() -> int:
    img, outdir = sys.argv[1], sys.argv[2]
    size = os.path.getsize(img)
    with open(img, "rb") as f:
        boot = f.read(512)
    if boot[510:512] != b"\x55\xaa":
        print("not a MBR/VBR start? continuing anyway")
    # BPB (FAT32 VBR at volume start for a superfloppy/image of the volume)
    bytes_per_sec = int.from_bytes(boot[11:13], "little")
    sec_per_clus = boot[13]
    clus_size = bytes_per_sec * sec_per_clus
    print(f"cluster_size={clus_size} bytes_per_sec={bytes_per_sec} spc={sec_per_clus}")
    data_off = 0  # carve the whole image; clusters found by pattern, not FAT walk
    hits = []  # (offset, line)
    with open(img, "rb") as f:
        chunk = 8 << 20
        off = 0
        prev = b""
        while off < size:
            f.seek(off)
            buf = f.read(chunk)
            if not buf:
                break
            buf = prev + buf
            base = off - len(prev)
            for m in LINE.finditer(buf):
                line_start = buf.rfind(b"\n", 0, m.start()) + 1
                line_end = buf.find(b"\n", m.start())
                if line_end < 0:
                    line_end = min(len(buf), m.start() + 400)
                line = buf[line_start:line_end]
                hits.append((base + line_start, line[:300]))
            off += chunk
            prev = b""
    # Deduplicate overlapping hits from chunk borders, stitch runs.
    hits.sort()
    sessions = []
    cur = []
    last_off = None
    for o, line in hits:
        if last_off is not None and o - last_off > clus_size * 4:
            if len(cur) >= MIN_RUN:
                sessions.append(cur)
            cur = []
        cur.append((o, line))
        last_off = o + len(line)
    if len(cur) >= MIN_RUN:
        sessions.append(cur)
    os.makedirs(outdir, exist_ok=True)
    print(f"total_hits={len(hits)} sessions={len(sessions)}")
    for i, s in enumerate(sessions):
        p = os.path.join(outdir, f"session-{i:02d}-off-{s[0][0]:x}.txt")
        with open(p, "wb") as g:
            for _, line in s:
                g.write(line + b"\n")
        print(f"{p}: {len(s)} lines, {s[0][0]:x}..{s[-1][0]:x}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
