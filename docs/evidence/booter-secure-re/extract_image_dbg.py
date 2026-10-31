#!/usr/bin/env python3
"""Extract NVIDIA booter IMAGE_DBG bindata, AES-ECB decrypt the GA-line
secure segment with the public debug test key, verify controls, persist.

Inputs:  open-gpu-kernel-modules generated bindata (trusted project input)
Outputs: <outdir>/<fam>_image_dbg.bin (raw), <fam>_sec_plain.bin (decrypted)
"""
import re, sys, zlib, hashlib, os, struct

# --- pure-python AES-128 (encrypt+decrypt, ECB) -----------------------------
SBOX = bytes.fromhex(
    "637c777bf26b6fc53001672bfed7ab76ca82c97dfa5947f0add4a2af9ca472c0"
    "b7fd9326363ff7cc34a5e5f171d8311504c723c31896059a071280e2eb27b275"
    "09832c1a1b6e5aa0523bd6b329e32f8453d100ed20fcb15b6acbbe394a4c58cf"
    "d0efaafb434d338545f9027f503c9fa851a3408f929d38f5bcb6da2110fff3d2"
    "cd0c13ec5f974417c4a77e3d645d197360814fdc222a908846eeb814de5e0bdb"
    "e0323a0a4906245cc2d3ac629195e479e7c8376d8dd54ea96c56f4ea657aae08"
    "ba78252e1ca6b4c6e8dd741f4bbd8b8a703eb5664803f60e613557b986c11d9e"
    "e1f8981169d98e949b1e87e9ce5528df8ca1890dbfe6426841992d0fb054bb16")
INV_SBOX = [0]*256
for i, v in enumerate(SBOX):
    INV_SBOX[v] = i
RCON = [0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80,0x1b,0x36]

def xtime(a):
    a <<= 1
    return (a ^ 0x1b) & 0xff if a & 0x100 else a

def key_expand(key):
    w = [list(key[i*4:i*4+4]) for i in range(4)]
    for i in range(4, 44):
        t = list(w[i-1])
        if i % 4 == 0:
            t = t[1:] + t[:1]
            t = [SBOX[b] for b in t]
            t[0] ^= RCON[i//4 - 1]
        w.append([w[i-4][j] ^ t[j] for j in range(4)])
    return [sum((w[4*r+c][j] << (24 - 8*j) for j in range(4)) for c in range(4)) for r in range(11)]

def add_rk(s, rk):
    for i in range(4):
        for j in range(4):
            s[i][j] ^= (rk >> (24 - 8*(4*j+i))) & 0xff if False else 0
    # (unused path)

def _xor_round(s, w, r):
    for c in range(4):
        for j in range(4):
            s[c][j] ^= w[4*r+c][j]

def _shift_rows(s):
    for r in range(1, 4):
        s[:]  # noqa
    # implemented in enc/dec bodies below

def aes_enc_block(key_exp, block):
    s = [[block[c*4+r] for c in range(4)] for r in range(4)]  # s[r][c]
    def addrk(r):
        for c in range(4):
            for j in range(4):
                s[j][c] ^= key_exp[4*r+c][j] if False else 0
    # simpler flat representation
    st = list(block)
    def xor_rk(r):
        for c in range(4):
            for j in range(4):
                st[4*c+j] ^= key_exp[4*r+c][j]
    def sub(): st[:] = [SBOX[b] for b in st]
    def shr():
        o = list(st)
        for c in range(4):
            for r in range(4):
                st[4*c+r] = o[4*((c+r) % 4)+r]
    def mix():
        for c in range(4):
            a = st[4*c:4*c+4]
            st[4*c+0] = xtime(a[0]) ^ xtime(a[1]) ^ a[1] ^ a[2] ^ a[3]
            st[4*c+1] = a[0] ^ xtime(a[1]) ^ xtime(a[2]) ^ a[2] ^ a[3]
            st[4*c+2] = a[0] ^ a[1] ^ xtime(a[2]) ^ xtime(a[3]) ^ a[3]
            st[4*c+3] = xtime(a[0]) ^ a[0] ^ a[1] ^ a[2] ^ xtime(a[3])
    xor_rk(0)
    for r in range(1, 10):
        sub(); shr(); mix(); xor_rk(r)
    sub(); shr(); xor_rk(10)
    return bytes(st)

def aes_dec_block(key_exp, block):
    st = list(block)
    def xor_rk(r):
        for c in range(4):
            for j in range(4):
                st[4*c+j] ^= key_exp[4*r+c][j]
    def inv_sub(): st[:] = [INV_SBOX[b] for b in st]
    def inv_shr():
        o = list(st)
        for c in range(4):
            for r in range(4):
                st[4*c+r] = o[4*((c-r) % 4)+r]
    def xtime_mul(a, n):
        r = 0
        for _ in range(n):
            r = xtime(r ^ (a & 1) * 0)  # placeholder, replaced below
        return r
    def gmul(a, b):
        p = 0
        for _ in range(8):
            if b & 1:
                p ^= a
            b >>= 1
            a = xtime(a)
        return p & 0xff
    def inv_mix():
        for c in range(4):
            a = st[4*c:4*c+4]
            st[4*c+0] = gmul(a[0],14) ^ gmul(a[1],11) ^ gmul(a[2],13) ^ gmul(a[3],9)
            st[4*c+1] = gmul(a[0],9) ^ gmul(a[1],14) ^ gmul(a[2],11) ^ gmul(a[3],13)
            st[4*c+2] = gmul(a[0],13) ^ gmul(a[1],9) ^ gmul(a[2],14) ^ gmul(a[3],11)
            st[4*c+3] = gmul(a[0],11) ^ gmul(a[1],13) ^ gmul(a[2],9) ^ gmul(a[3],14)
    xor_rk(10)
    for r in range(9, 0, -1):
        inv_shr(); inv_sub(); xor_rk(r); inv_mix()
    inv_shr(); inv_sub(); xor_rk(0)
    return bytes(st)

def expand(key):
    """Real AES-128 key expansion -> 44 words, ke[4*r+c] = word(4 bytes)."""
    w = [list(key[i*4:i*4+4]) for i in range(4)]
    for i in range(4, 44):
        t = list(w[i-1])
        if i % 4 == 0:
            t = t[1:] + t[:1]
            t = [SBOX[b] for b in t]
            t[0] ^= RCON[i//4 - 1]
        w.append([w[i-4][j] ^ t[j] for j in range(4)])
    return w

# --- bindata parsing ---------------------------------------------------------
ARR_RE = re.compile(
    r"BINDATA_LABEL_(\w+)_data\[\]\s*=\s*\{(.*?)\}", re.S)
NUM_RE = re.compile(r"0x([0-9a-fA-F]+)ULL")

def load_arrays(path):
    txt = open(path, encoding="utf8", errors="replace").read()
    out = {}
    for label, body in ARR_RE.findall(txt):
        vals = [int(m, 16) for m in NUM_RE.findall(body)]
        blob = b"".join(struct.pack("<Q", v) for v in vals)
        out[label] = blob
    return out

def raw_inflate(blob, want):
    for trim in range(0, 16):
        try:
            d = zlib.decompressobj(-15)
            data = d.decompress(blob[:len(blob)-trim]) + d.flush()
            if len(data) == want:
                return data
        except Exception:
            continue
    raise RuntimeError("inflate failed")

def main():
    gen = sys.argv[1]
    outdir = sys.argv[2]
    fams = ["GA102", "AD102", "GA100", "TU102"]
    key = bytes.fromhex("REDACTED-GA-DEBUG-KEY-SEE-GIST")
    ke = expand(key)
    # AES sanity: encrypt/decrypt roundtrip + published controls
    enc0 = aes_enc_block(ke, bytes(16))
    assert enc0 == bytes.fromhex("717d1494eaca317ff106195258b38377"), enc0.hex()
    assert aes_dec_block(ke, enc0) == bytes(16)
    assert aes_enc_block(ke, b"\xff"*16) == bytes.fromhex("fd7e5b57768a309baabcc6903118c8f5")
    print("[+] AES controls PASS (00->717d1494.., ff->fd7e5b57..)")
    for fam in fams:
        p = os.path.join(gen, f"g_bindata_kgspGetBinArchiveBooterLoadUcode_{fam}.c")
        arrs = load_arrays(p)
        img = arrs.get("IMAGE_DBG")
        hd = arrs.get("HEADER_DBG")
        if img is None:
            print(f"[-] {fam}: no IMAGE_DBG"); continue
        txt = open(p, encoding="utf8", errors="replace").read()
        m = re.search(r"LABEL_IMAGE_DBG\b.*?DATA SIZE \(bytes\): (\d+)", txt, re.S)
        want = int(m.group(1))
        data = raw_inflate(img, want)
        open(os.path.join(outdir, f"{fam}_image_dbg.bin"), "wb").write(data)
        if hd is None:
            print(f"[-] {fam}: no HEADER_DBG"); continue
        m = re.search(r"LABEL_HEADER_DBG\b.*?DATA SIZE \(bytes\): (\d+)", txt, re.S)
        hwant = int(m.group(1))
        hdata = raw_inflate(hd, hwant)
        f = struct.unpack("<%dI" % (hwant // 4), hdata)
        print(f"[+] {fam}: IMAGE {len(data)}B  hdr{hwant}B f={f}")
        # field semantics = BTL1 h(i): f0=ns_off f1=ns_size f2=data_off f3=data_size
        # f4=? f5=sec(app_code)_off f6=sec_size ...
        ns_off, ns_size, d_off, d_size = f[0], f[1], f[2], f[3]
        sec_off, sec_size = f[5], f[6]
        if fam in ("GA102", "AD102", "GA100"):
            ct = data[sec_off:sec_off + sec_size]
            pad = (-len(ct)) % 16
            ct = ct + b"\x00" * pad
            pt = b"".join(aes_dec_block(ke, ct[i:i+16]) for i in range(0, len(ct), 16))[:sec_size]
            fn = os.path.join(outdir, f"{fam}_sec_plain.bin")
            open(fn, "wb").write(pt)
            import math
            from collections import Counter
            c = Counter(pt); n = len(pt)
            ent = -sum(v / n * math.log2(v / n) for v in c.values())
            blocks = Counter(pt[i:i+16] for i in range(0, len(pt) - 15, 16))
            pad_top = blocks.most_common(2)
            print(f"    ns=[{ns_off:#x},{ns_off+ns_size:#x}) sec=[{sec_off:#x},{sec_off+sec_size:#x}) dmem=[{d_off:#x},{d_off+d_size:#x})")
            print(f"    sec plaintext {len(pt)}B entropy={ent:.3f} sha256={hashlib.sha256(pt).hexdigest()}")
            print(f"    top blocks: {[(b.hex(), c_) for b, c_ in pad_top]}")

if __name__ == "__main__":
    main()
