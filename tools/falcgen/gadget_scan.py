# HISTORICAL PROTOTYPE (superseded by the Rust scanner in src/scan.rs).
# Kept as the G0 feasibility evidence: 4/6 TU102 + 2/5 GA102 recovery with
# a harvested length table; the misses (mid-entry opcode starts absent
# from the canonical stream) are exactly the gap the Rust decoder closed.
# Run: GADGET_LST=<sec.lst> python -I gadget_scan.py <sec_plain.bin> 0x100 [known VAs...]
# First-principles gadget scanner prototype (research, issue-#151 follow-on).
#
# Question: can the payload's gadget set be DERIVED from the decrypted booter
# segment alone? Golden standard: the scanner must rediscover the six TU102
# sites the upstream tables reference (0x10B9 write mid-entry, 0x1F89 guard,
# 0xCBD/0xC95/0xE85 mutex, 0x7EA4 park) without being told them.
#
# Method: bootstrap a fuc5 instruction-length table from the existing envydis
# lst (address -> bytes), then DFS-decode from EVERY byte offset (both
# parities - mid-entry gadgets are odd-aligned), classify forward slices that
# reach a terminator, rank by primitive interest.
# Usage: python -I gadget_scan.py <sec_plain.bin> <va_base> [known sites hex...]
import sys, re, collections

import os
LST = os.environ.get("GADGET_LST", r"D:/git-repo/nv-uefi-re/TU102_sec.lst")

def harvest_lengths(path):
    lens = collections.defaultdict(collections.Counter)
    pat = re.compile(r"^([0-9a-f]{8}): ((?:[0-9a-f]{2} )+) ", re.M)
    for m in pat.finditer(open(path, encoding="utf-8", errors="replace").read()):
        bs = bytes.fromhex(m.group(2).replace(" ", ""))
        if bs:
            lens[bs[0]][len(bs)] += 1
    return lens

def build_decoder(lens):
    # first byte -> candidate lengths (most common first)
    return {b: [l for l, _ in c.most_common()] for b, c in lens.items()}

TERM_OPS = (0xFB,)   # fb = mpopaddret family (ret-class); 3e = lbra handled by imm
INTERESTING = "iowrs st imm-pri lcall lbra"

def decode_from(data, off, table, max_ins=10):
    """Linear decode up to max_ins instructions from off (most-common length
    per first byte). Returns the instruction list, or None if it derails."""
    seq = []
    p = off
    for _ in range(max_ins):
        if p >= len(data):
            return None
        op = data[p]
        if op not in table:
            return None
        ln = table[op][0]
        if p + ln > len(data):
            return None
        seq.append((p, ln, op))
        p += ln
        # ROP terminators: ret-class, absolute tail-call, conditional-less
        # branch. lcall counts too - the callee returns into OUR next frame.
        if op in (0xFB, 0x3E, 0x7E):
            return seq
    return None                                  # no terminator in budget

def classify(data, off, seq, va_base):
    """Crude interest classification of one decoded slice."""
    tags = set()
    va = va_base + off
    for (p, ln, op) in seq:
        w = data[p:p+ln]
        if op == 0xF7:                            # iowrs I[..] $rN - MMIO write
            tags.add("mmio-write")
        if op in (0xA0, 0xA1):                    # st D[..] - DMEM store
            tags.add("dmem-store")
        if op == 0x8F and ln >= 4:                # mov $r15, imm
            imm = int.from_bytes(w[2:4], "little")
            if 0x1000 <= imm < 0x10000:           # DMEM-ish immediate
                tags.add(f"dmem-imm:{hex(imm)}")
            if imm >= 0x100000:                   # PRI-ish immediate
                tags.add("pri-imm")
        if op == 0x7E:
            tags.add("lcall")
        if op == 0x3E:
            imm = int.from_bytes(w[1:4], "little")
            if imm == va:
                tags.add("park-self")
            else:
                tags.add("lbra")
        if op == 0xFB:
            tags.add("ret")
    return tags

def main():
    data = open(sys.argv[1], "rb").read()
    va_base = int(sys.argv[2], 0)
    known = [int(x, 0) for x in sys.argv[3:]]
    lens = harvest_lengths(LST)
    multi = {b: dict(c) for b, c in lens.items() if len(c) > 1}
    print(f"length table: {len(lens)} first-bytes, {len(multi)} ambiguous: "
          + ", ".join(f"{hex(b)}:{v}" for b, v in list(multi.items())[:6]))
    table = build_decoder(lens)

    cands = {}
    for off in range(len(data)):
        seq = decode_from(data, off, table)
        if not seq:
            continue
        tags = classify(data, off, seq, va_base)
        if tags & {"mmio-write", "park-self", "ret", "lcall", "dmem-imm"}:
            score = len(tags)
            cands[off + va_base] = (score, sorted(tags))

    print(f"candidates: {len(cands)}")
    for va in known:
        if va - va_base < len(data):
            hit = cands.get(va)
            print(f"  known {hex(va)}: {'HIT ' + str(hit) if hit else 'miss'}")
    # top-ranked by tag count, then address
    top = sorted(cands.items(), key=lambda kv: (-kv[1][0], kv[0]))[:25]
    print("top candidates:")
    for va, (score, tags) in top:
        print(f"  {va:#07x} score={score} {tags}")

if __name__ == "__main__":
    main()
