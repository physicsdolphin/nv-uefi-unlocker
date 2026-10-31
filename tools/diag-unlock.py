#!/usr/bin/env python3
"""NVIDIA unlock state reader / re-applier (host-OS side, Linux).

Reads the FEAT cluster straight off BAR0 via sysfs (no driver needed):
mmap /sys/bus/pci/devices/<bdf>/resource0 and read dwords. Root required.

Families (PMC_BOOT_0 bits [31:20]):
  Turing  (0x160..0x16F)  FEAT_PLM 0x00409650 / SS0 0x00409664 / SS1 0x0040966C / READOUT 0x00409668
  Ampere/Ada (0x170..0x17F, 0x220..0x22F, 0x100)  0x00823804 / 0x0082381C / 0x00823820
  (+ READOUT_1 0x00823818 on the 0x823xxx block; 0x00409668 on Turing, per
  nlzy/turing-unlock - same constant ssdiag carries, issue #42)

Why this exists (issue #6): the EFI unlock's volatile state is only useful if
it survives into the OS. This tool answers, from inside the running system:
  - did the SS override values persist?           (unlock state)
  - is the FEAT PLM gate still open?              (re-apply possible?)
`--reapply` rewrites SS0/SS1 while the door is open - the "separate-lock"
method: devinit may re-derive the rates from fuses (it holds L3), but with
the gate left open by the EFI chain the host can simply write them again.

Usage:
  sudo python3 diag-unlock.py                 # read all NVIDIA dGPUs
  sudo python3 diag-unlock.py --bdf 0000:01:00.0
  sudo python3 diag-unlock.py --reapply       # SS0=0x88888888 SS1=0x8 when open
"""

import argparse
import mmap
import os
import sys

SS0_UNLOCK = 0x8888_8888
SS1_UNLOCK = 0x8

# offset -> name, per family
TURING = {"plm": 0x0040_9650, "ss0": 0x0040_9664, "readout": 0x0040_9668, "ss1": 0x0040_966C}
GAADA = {
    "plm": 0x0082_3804,
    "ss0": 0x0082_381C,
    "ss1": 0x0082_3820,
    "readout": 0x0082_3818,
}
# union with the reference unlockers (loader M1 + cmp170hx survey + chenxluo)
CORE_EXTRA = [
    ("VGA_WORKSPACE", 0x0062_5F04),
    ("MMU_PLM", 0x001F_A7C8),
    ("FUSE_VER_DECODE", 0x0082_41C0),
]
ADA_ROWS = [
    ("WPR_PLM", 0x001F_A7C4),
    ("WPR_CFG_PLM", 0x001F_A7CC),
    ("FBPA_PLM", 0x009A_0148),
]
AD1X_ROWS = [
    ("AD1X_CYA", 0x0008_8724),
    ("AD1X_CFG", 0x0008_8DCC),
    ("AD1X_XVE_PLM", 0x0008_8FF4),
    ("AD1X_BAR1_CAP_MIRROR", 0x0008_8BBC),
]
PLM_SURVEY = [
    0x0082_00D0, 0x0082_00D4, 0x0082_00D8, 0x0082_00DC, 0x0082_00E0, 0x0082_00E4,
    0x0082_00E8, 0x0082_00EC, 0x0082_00F0, 0x0082_00F4, 0x0082_00FC,
    0x0082_3800, 0x0082_3804, 0x0082_3B00,
    0x009A_0008, 0x009A_000C, 0x009A_0148, 0x009A_014C, 0x009A_0168, 0x009A_03F0,
    0x009A_0554, 0x009A_0BFC,
    0x0010_0B10, 0x0010_0B38, 0x0010_0B84, 0x0010_0B9C,
]
AMPADA_SWEEP = [
    0x0082_3800, 0x0082_3808, 0x0082_380C, 0x0082_3810, 0x0082_3814, 0x0082_3824,
    0x0082_3828, 0x0082_382C, 0x0082_3B00,
]


def find_gpus():
    out = []
    root = "/sys/bus/pci/devices"
    for bdf in sorted(os.listdir(root)):
        p = os.path.join(root, bdf)
        try:
            vendor = open(os.path.join(p, "vendor")).read().strip()
            cls = open(os.path.join(p, "class")).read().strip()
        except OSError:
            continue
        if vendor != "0x10de" or not cls.startswith("0x03"):
            continue
        # function 0 only (the GPU, not its audio/USB-C companions)
        if ".0" not in bdf and "0000:" in bdf and not bdf[-2] == ".":
            continue
        out.append((bdf, p))
    return out


class Bar0:
    def __init__(self, devpath):
        with open(os.path.join(devpath, "resource")) as f:
            start = int(f.readline().split()[0], 16)
        if start == 0:
            raise RuntimeError("BAR0 not assigned (device powered down?)")
        self.start = start
        f = open(os.path.join(devpath, "resource0"), "r+b", 0)
        self.map = mmap.mmap(f.fileno(), 16 * 1024 * 1024)

    def rd(self, off):
        return int.from_bytes(self.map[off : off + 4], "little")

    def wr(self, off, val):
        self.map[off : off + 4] = int.to_bytes(val, 4, "little")


def family_of(boot0):
    # die = bits [31:20]; nvchip series per nv_arch.h GPU_ARCHITECTURE table
    # (MAXWELL=0x110 PASCAL=0x130 VOLTA=0x140 TURING=0x160 AMPERE=0x170
    #  HOPPER=0x180 ADA=0x190 GB1XX=0x1A0 GB2XX=0x1B0 RUBIN=0x1C0)
    # field anchors: TU106=0x166 AD106=0x196 AD107=0x197 GA100=0x170 GA106=0xb76
    die = (boot0 >> 20) & 0xFFF
    if 0x110 <= die <= 0x11F:
        return "maxwell", None
    if 0x120 <= die <= 0x12F:
        return "maxwell2", None
    if 0x130 <= die <= 0x13F:  # GP104=0x134 GP106=0x136
        return "pascal", None
    if 0x140 <= die <= 0x14F:  # GV100=0x140
        return "volta", None
    if 0x150 <= die <= 0x15F:
        return "volta2", None
    if 0x160 <= die <= 0x16F:
        return "turing", TURING
    if 0x170 <= die <= 0x17F:  # GA100 reads 0x170000a1 plainly
        return "ampere", GAADA
    if 0x180 <= die <= 0x18F:  # GH100=0x180
        return "hopper", None
    if 0x190 <= die <= 0x19F:  # Ada is NV19x, NOT 0x22x (device ids)
        return "ada", GAADA
    if 0x1A0 <= die <= 0x1AF:  # GB102=0x1A2
        return "blackwell-gb1", None
    if 0x1B0 <= die <= 0x1BF:  # GB202-class (50-series)
        return "blackwell-gb2", None
    if 0x1C0 <= die <= 0x1CF:
        return "rubin", None
    if 0xB70 <= die <= 0xB7F:  # GA10x quirk: 0xb7xx00a1 (GA102=b74, GA106=b76)
        return "ampere", GAADA
    return None, None


def decode_plm(v):
    if v == 0xFFFF_FFFF:
        return "OPEN"
    if v == 0xFFFF_FF8F:
        return "closed (stock)"
    if v == 0xFFFF_FFEF:
        return "closed (relock+drift)"
    return "closed (non-standard)"


def decode_ss(v, width):
    # 4-bit fields: bit3 = override-enable, bits[2:0] = rate (0 full 1 half 2 quarter 3 eighth)
    names = {0: "full", 1: "1/2", 2: "1/4", 3: "1/8"}
    fields = []
    for i in range(width):
        nib = (v >> (4 * i)) & 0xF
        if nib == 0:
            fields.append(".")
        elif nib & 0x8:
            fields.append(names.get(nib & 7, "?") + "*")
        else:
            fields.append("r%d" % (nib & 7))
    return " ".join(fields)


def report(bar, tag=""):
    boot0 = bar.rd(0x0)
    fam, regs = family_of(boot0)
    if fam is None:
        if boot0 in (0, 0xFFFFFFFF):
            state = "all zeros" if boot0 == 0 else "all ones"
            print(f"  PMC_BOOT_0={boot0:#010x} [{state}] - device likely D3 / MEM decode off; nothing to diagnose")
        else:
            print(f"  PMC_BOOT_0={boot0:#010x} - NVIDIA-class unknown die; diagnosis-only pass (FEAT addresses unmapped)")
        return None
    if regs is None:
        print(f"  VERDICT: family recognized - FEAT addresses not mapped for this family (diagnosis only)")
        return None
    plm = bar.rd(regs["plm"])
    ss0 = bar.rd(regs["ss0"])
    ss1 = bar.rd(regs["ss1"])
    print(f"  PMC_BOOT_0={boot0:#010x} family={fam} {tag}")
    print(f"  FEAT_PLM  = {plm:#010x} ({decode_plm(plm)})")
    print(f"  FEAT_SS0  = {ss0:#010x} [{decode_ss(ss0, 8)}]")
    print(f"  FEAT_SS1  = {ss1:#010x} [{decode_ss(ss1, 1)}]")
    if "readout" in regs:
        ro = bar.rd(regs["readout"])
        note = "PLM-blocked" if (ro & 0xFFFF0000) == 0xBADF0000 else ("full speed" if ro == 0 else "")
        print(f"  READOUT_1 = {ro:#010x} {note}")
    for name, off in CORE_EXTRA:
        print(f"  {name:<22} = {bar.rd(off):#010x}")
    print("  --- family-specific / survey (badf on the wrong die is expected) ---")
    for name, off in ADA_ROWS + AD1X_ROWS:
        v = bar.rd(off)
        note = f" ({decode_plm(v)})" if name.endswith("PLM") else ""
        print(f"  {name:<22} = {v:#010x}{note}")
    for i in range(0, len(PLM_SURVEY), 4):
        row = PLM_SURVEY[i:i+4]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    print("  --- ampere/ada FEAT sweep ---")
    for i in range(0, len(AMPADA_SWEEP), 5):
        row = AMPADA_SWEEP[i:i+5]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    unlocked = ss0 == SS0_UNLOCK and ss1 == SS1_UNLOCK
    print(f"  VERDICT: {'UNLOCKED (override values live)' if unlocked else 'not unlocked'}"
          f"{' - reapply available (door open)' if not unlocked and plm == 0xFFFF_FFFF else ''}")
    return fam, regs, plm, unlocked


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bdf", help="only this PCI address, e.g. 0000:01:00.0")
    ap.add_argument("--reapply", action="store_true",
                    help="write SS0=0x88888888 SS1=0x8 where the PLM gate is open")
    args = ap.parse_args()

    if os.geteuid() != 0:
        print("root required (sysfs BAR0 mmap)", file=sys.stderr)
        return 2

    devs = find_gpus()
    if args.bdf:
        devs = [(b, p) for (b, p) in devs if b.endswith(args.bdf) or b.split(":")[-1] == args.bdf.split(":")[-1]]
    if not devs:
        print("no NVIDIA display controller found", file=sys.stderr)
        return 1

    rc = 0
    for bdf, devpath in devs:
        print(f"{bdf}:")
        try:
            bar = Bar0(devpath)
        except (PermissionError, RuntimeError) as e:
            print(f"  BAR0 unavailable: {e}")
            rc = 1
            continue
        st = report(bar)
        if st is None:
            continue
        fam, regs, plm, unlocked = st
        if args.reapply:
            if plm != 0xFFFF_FFFF:
                print("  reapply REFUSED: FEAT PLM gate is closed - the EFI chain owns door-opening;"
                      " re-run the USB unlock instead")
                rc = 1
            else:
                bar.wr(regs["ss0"], SS0_UNLOCK)
                bar.wr(regs["ss1"], SS1_UNLOCK)
                rb0, rb1 = bar.rd(regs["ss0"]), bar.rd(regs["ss1"])
                ok = rb0 == SS0_UNLOCK and rb1 == SS1_UNLOCK
                print(f"  reapply: SS0={rb0:#010x} SS1={rb1:#010x} -> {'OK' if ok else 'READBACK MISMATCH'}"
                      " (read-back is the only success criterion)")
                rc = 0 if ok else 1
    return rc


if __name__ == "__main__":
    sys.exit(main())
