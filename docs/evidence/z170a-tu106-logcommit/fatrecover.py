#!/usr/bin/env python3
"""Proper FAT32 recovery of loader.log from a raw volume image: VBR -> BPB
-> root dir -> LOADER.LOG dirent -> FAT chain walk -> dump. Also reports
the dirent's size/first-cluster and raw search hits for new-era markers.

usage: python -I fatrecover.py <image> <out-file>
"""
import struct
import sys


def main() -> int:
    img, out = sys.argv[1], sys.argv[2]
    f = open(img, "rb")

    vbr = f.read(512)
    bps = struct.unpack("<H", vbr[11:13])[0]
    spc = vbr[13]
    reserved = struct.unpack("<H", vbr[14:16])[0]
    nfats = vbr[16]
    fatsz = struct.unpack("<I", vbr[36:40])[0]
    root_clus = struct.unpack("<I", vbr[44:48])[0]
    fat_off = reserved * bps
    data_off = (reserved + nfats * fatsz) * bps
    clus_bytes = bps * spc
    print(f"bps={bps} spc={spc} reserved={reserved} nfats={nfats} fatsz={fatsz} "
          f"root={root_clus:#x} data_off={data_off:#x} cluster={clus_bytes}")

    def fat_entry(n: int) -> int:
        f.seek(fat_off + n * 4)
        return struct.unpack("<I", f.read(4))[0] & 0x0FFFFFFF

    def read_chain(first: int) -> bytes:
        outb = bytearray()
        c = first
        seen = set()
        while 2 <= c < 0x0FFFFFF8 and c not in seen:
            seen.add(c)
            f.seek(data_off + (c - 2) * clus_bytes)
            outb += f.read(clus_bytes)
            c = fat_entry(c)
        return bytes(outb)

    def list_dir(clus: int):
        data = read_chain(clus)
        for i in range(0, len(data), 32):
            e = data[i:i + 32]
            if len(e) < 32 or e[0] == 0:
                break
            if e[0] == 0xE5 or e[11] & 0x08:  # deleted or LFN
                continue
            name = e[0:8].decode("ascii", "replace").rstrip() 
            ext = e[8:11].decode("ascii", "replace").rstrip()
            attr = e[11]
            first = (struct.unpack("<H", e[20:22])[0] << 16) | struct.unpack("<H", e[26:28])[0]
            size = struct.unpack("<I", e[28:32])[0]
            if attr & 0x10:  # directory
                if name not in (".", ".."):
                    yield (name, None, first, size, True)
            else:
                yield (f"{name}.{ext}".rstrip("."), attr, first, size, False)

    def find(path_parts, clus):
        for name, _, first, size, isdir in list_dir(clus):
            if name.upper() == path_parts[0].upper():
                if len(path_parts) == 1:
                    return first, size
                if isdir:
                    return find(path_parts[1:], first)
        return None

    hit = find(["LOADER.LOG"], root_clus)
    if not hit:
        print("LOADER.LOG dirent not found in root")
        return 1
    first, size = hit
    print(f"LOADER.LOG dirent: size={size} first_cluster={first:#x}")

    chain = []
    c = first
    while 2 <= c < 0x0FFFFFF8 and c not in chain:
        chain.append(c)
        c = fat_entry(c)
    print(f"chain length={len(chain)} clusters: {chain[:12]}{'...' if len(chain) > 12 else ''}")
    data = read_chain(first)
    with open(out, "wb") as g:
        g.write(data)
    # The dirent says size=0; dump the whole chain but trim to plausible
    # content: cut at the first run of >=4 KiB of zeros past 4 KiB.
    trim = len(data)
    run = 0
    for i in range(4096, len(data), 512):
        if data[i:i + 512] == b"\0" * 512:
            run += 512
            if run >= 4096:
                trim = i + 512 - run
                break
        else:
            run = 0
    with open(out, "wb") as g:
        g.write(data[:trim])
    print(f"chain bytes={len(data)} trimmed_to={trim} -> {out}")

    # Raw search: new-era markers anywhere on the medium.
    markers = [b"BOOT: logger init returned", b"EARLY", b"IDENTITY", b"NATIVE-GOP: shadowed"]
    f.seek(0)
    chunk = 16 << 20
    off = 0
    prev = b""
    counts = {m: 0 for m in markers}
    first_off = {m: None for m in markers}
    while True:
        buf = f.read(chunk)
        if not buf:
            break
        scan = prev + buf
        base = off - len(prev)
        for m in markers:
            idx = 0
            while True:
                i = scan.find(m, idx)
                if i < 0:
                    break
                counts[m] += 1
                if first_off[m] is None:
                    first_off[m] = base + i
                idx = i + 1
        off += len(buf)
        prev = buf[-64:]
    for m in markers:
        print(f"marker {m[:30]!r}: count={counts[m]} first_off={first_off[m]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
