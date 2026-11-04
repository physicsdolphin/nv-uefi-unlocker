#!/usr/bin/env python3
"""NVIDIA unlock state reader / re-applier (host-OS side, Linux).

Reads the FEAT cluster straight off BAR0 via sysfs (no driver needed):
mmap /sys/bus/pci/devices/<bdf>/resource0 and read dwords. Root required.

Families (PMC_BOOT_0 bits [31:20]):
  Turing  (0x160..0x16F)  FEAT_PLM 0x00409650 / SS0 0x00409664 / SS1 0x0040966C / READOUT 0x00409668
  Ampere/Ada (0x170..0x17F, 0x220..0x22F, 0x100)  0x00823804 / 0x0082381C / 0x00823820
  (+ READOUT_1 0x00823818 on the 0x823xxx block; 0x00409668 on Turing, per
  nlzy/turing-unlock - same constant ssdiag carries, issue #42)
  Blackwell (0x1A0..0x1BF, plus 0x210..0x21F for the GB202 RM-dump form)
  shares the 0x823xxx block and additionally prints
  its fuse selectors. Blackwell is READ-ONLY: no SEC2 stage, no GB20x booter,
  so the PLM can only be opened by PL3 firmware and --reapply always refuses
  (issue #168, docs/blackwell-gb20x-status.md). READOUT_1 is decoded with the
  real non-contiguous bit offsets as fields - Blackwell 3 is ~half rate, not
  the Ada 1/8.

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
# Blackwell GB20x uses the same 0x823xxx bank (issue #168, GB206 10de:2d04:
# PLM 0xfffffb8f, SS0 0x06277073, SS1 0x00025005, READOUT_1 0x00c36180).
# Separate table because the readout decode and the rate law differ, and no
# transaction can run on the die (no GB20x SEC2 booter).
BLACKWELL = dict(GAADA)
BLACKWELL_ROWS = [
    ("FUSE_EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSE_SS_PLM", 0x0082_00FC),
    ("SECURE_FEAT_OVR_QUADRO_WR", 0x0082_038C),
    ("FUSE_SS_DP", 0x0082_0224),
    ("FUSE_SS_FFMA", 0x0082_059C),
    ("FUSE_SS_FMLA16", 0x0082_07D4),
    ("FUSE_SS_FMLA32", 0x0082_07D8),
    ("FUSE_SS_IMLA0", 0x0082_07DC),
    ("FUSE_SS_IMLA1", 0x0082_07E0),
    ("FUSE_SS_IMLA2", 0x0082_07E4),
    ("FUSE_SS_IMLA3", 0x0082_07E8),
    ("FUSE_SS_IMLA4", 0x0082_07EC),
]
# FEAT_READOUT_1 is NOT nine contiguous 3-bit fields: offsets 0,3,7,10,13,
# 16,19,22,28 (gaps at bit 6 and bits 27:25). Contiguous decode yields
# out-of-range 6s. Field order imla0 fmla16 fmla32 ffma imla1 imla2 imla3
# imla4 DP.
READOUT1_SHIFT = [0, 3, 7, 10, 13, 16, 19, 22, 28]
# Core/debug block, synced to ssdiag 0.6.4 (CORE_ROWS): the 2026-11-04 sync
# brings the python twin to the ssdiag 0.5/0.6 probe surface - 26-register
# PLM survey was already here; the PCIe/XP3G/OPT/Ada-fuse tables, the
# full core block and the badf classification are new.
# Reset canary (issues #164/#165), twin of ssdiag v0.6.5: the loader writes
# these two PBUS scratch words after its FLR; any reset clears them, and no
# driver pass writes this pair. Read with the FEAT values - canary intact +
# SS gone = a plain write; cleared + SS live = FLR/SBR-class reset; both gone
# = rail-off / full-chip reset.
CANARY_A = 0x0000_1480
CANARY_B = 0x0000_1484
CANARY_MAGIC_A = 0x0164_C0DE
CANARY_MAGIC_B = 0x0165_CAFE


def canary_state(bar):
    a, b = bar.rd(CANARY_A), bar.rd(CANARY_B)
    poison = lambda v: v == 0xFFFF_FFFF or v >> 16 == 0xBADF
    if poison(a) and poison(b):
        return "unreadable"
    if a == CANARY_MAGIC_A and b == CANARY_MAGIC_B:
        return "intact"
    if a == 0 and b == 0:
        return "cleared"
    if (a == CANARY_MAGIC_A) != (b == CANARY_MAGIC_B):
        return "partial"
    return "other"


CORE_ROWS = [
    ("PMC_BOOT_0", 0x0000_0000),
    ("PSTRAPS_SET0", 0x0010_1000),
    ("PSTRAPS_SET1", 0x0010_100C),
    ("FB_SIZE_MIB", 0x0011_83A4),
    ("GFW_PLM", 0x0011_8128),
    ("GFW_SCRATCH", 0x0011_8234),
    ("BSI_SCRATCH15", 0x0011_80FC),
    ("VGA_WORKSPACE", 0x0062_5F04),
    ("WPR2_LO", 0x001F_A824),
    ("WPR2_HI", 0x001F_A828),
    ("MMU_PLM", 0x001F_A7C8),
    ("MMU_LOCK_LO", 0x001F_A82C),
    ("MMU_LOCK_HI", 0x001F_A830),
    ("GSP_CPUCTL", 0x0011_0100),
    ("GSP_EXCI", 0x0011_00D0),
    ("SEC2_CPUCTL", 0x0084_0100),
    ("SEC2_MBOX0", 0x0084_0040),
    ("SEC2_MBOX1", 0x0084_0044),
    ("SEC2_EXCI", 0x0084_00D0),
    ("FUSE_VER_DECODE", 0x0082_41C0),
]
PRIV_ROWS = [("FUSE_OPT_FEATURE_OVR", 0x0082_03F0), ("FUSE_VER", 0x0082_4140)]
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
# Ada fuse-controller / TPC-disable block (ssdiag 0.6.4, KohakUnlock #16):
# badf on non-Ada dies is expected and is data.
ADA_FUSE_ROWS = [
    ("FUSE_CTRL", 0x0082_0000),
    ("FUSE_ADDR", 0x0082_0004),
    ("FUSE_REG_50", 0x0082_0050),
    ("FUSE_REG_54", 0x0082_0054),
    ("FUSE_DEBUGCTRL", 0x0082_0060),
    ("DISABLE_FUSE_PROG", 0x0082_007C),
    ("TPC_DEFECT_G0T0A", 0x0082_03A4),
    ("TPC_DEFECT_G0T0B", 0x0082_03AC),
    ("TPC_DEFECT_G0T0C", 0x0082_05C4),
    ("TPC_DEFECT_G0T0D", 0x0082_05C8),
    ("TPC_OPT_G0T0", 0x0082_0C38),
    ("TPC_OPT_G1T0", 0x0082_0C3C),
    ("CTRL_OPT_9C8", 0x0082_09C8),
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
# PCIe link block (ssdiag field round 3 union: cmp50hx patch04 + cmpunlocker
# 0007 + gpu-unlock-bar1 + AD102 L2 research). XVE shadow base 0x88000.
PCIE_LINK_ROWS = [
    ("LNKCAP", 0x0008_8084),
    ("LNKCTL_LNKSTA", 0x0008_8088),
    ("LNKCAP2", 0x0008_80A4),
    ("LNKCTL2_LNKSTA2", 0x0008_80A8),
    ("PRIV_MISC_1", 0x0008_841C),
    ("VSEC_DEVICE", 0x0008_860C),
    ("VSEC_HIERARCHY", 0x0008_8610),
    ("XVE_OVR_LTSSM", 0x0008_872C),
    ("XVE_B_PLM", 0x0008_8AB4),
    ("XVE_D0_PLM", 0x0008_8FE8),
    ("XVE_D4_PLM", 0x0008_8FEC),
    ("XVE_D8_PLM", 0x0008_8FF0),
    ("XVE_C_PLM", 0x0008_8FF8),
    ("LINK_CONFIG_0", 0x0008_C040),
    ("PL_LINK_RATE", 0x0008_C1C0),
    ("CYA_0_DISG2", 0x0008_C2C0),
]
# XP3G fuse-override block: the Gen2 capability layer - XP3G_PLM0 must read
# OPEN before any policy write can land.
XP3G_ROWS = [
    ("XP3G_STATUS0", 0x0008_E100),
    ("XP3G_STATUS3", 0x0008_E10C),
    ("XP3G_OVR0", 0x0008_E110),
    ("XP3G_OVR3", 0x0008_E11C),
    ("XP3G_VAL0", 0x0008_E120),
    ("XP3G_VAL3", 0x0008_E12C),
    ("XP3G_PLM0", 0x0008_E1B0),
    ("XP3G_PLM4", 0x0008_E1B4),
    ("XP3G_PLM8", 0x0008_E1B8),
    ("XP3G_PLMC", 0x0008_E1BC),
]
# Fuse/OPT region union: the Gen2 fuse wall, the GPC-disable shadow, the
# AD102 L2 disable-mask layer (effective = fuse layer | SW mask).
OPT_FUSE_ROWS = [
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("OPT_MAGIC", 0x0082_0520),
    ("OPT_GEN23", 0x0082_057C),
    ("OPT_GEN3", 0x0082_0580),
    ("L2_CANDIDATE", 0x0082_1200),
    ("L2_SW_MASK", 0x0082_2080),
    ("L2_EFF_MASK", 0x0082_2900),
]
# cmpunlocker PR#55's GA100 per-GPC SM override/status banks. On 20-40
# series expect badf - the existence/nonexistence answer per die, read-only.
GA100_SM_OVR_BASE = 0x0082_0A40
GA100_SM_STATUS_BASE = 0x0082_0C38
GA100_GPCS = 8


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
    if 0x1A0 <= die <= 0x1AF:  # GB100=0x1A0, GB102=0x1A2
        return "blackwell-gb1", BLACKWELL
    if 0x1B0 <= die <= 0x1BF:  # GB202/GB203M/GB206 class (50-series)
        return "blackwell-gb2", BLACKWELL
    if 0x210 <= die <= 0x21F:  # GB202 also reports 0x21120000 in an RM debug dump
        return "blackwell-gb2", BLACKWELL
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
    if v == 0xFFFF_FB8F:
        return "closed (PL3-only writes)"
    if v == 0xFFFF_FFEF:
        return "closed (relock+drift)"
    return "closed (non-standard)"


def decode_readout1(v):
    """Fields, not rates. Blackwell 3 measures ~half rate, not the Ada 1/8."""
    return " ".join(str((v >> s) & 7) for s in READOUT1_SHIFT)


def badf_bucket(v):
    """ssdiag 0.6.x classification: 0xBADF5040 privilege violation,
    0xBADF1100 undecoded, 0xBADF20xx floorswept, all-ones no decode."""
    if v == 0xFFFF_FFFF:
        return "NO_DECODE"
    if (v >> 16) != 0xBADF:
        return None
    lo = v & 0xFFFF
    if v == 0xBADF_5040:
        return "PRIV_VIOLATION"
    if lo == 0x1100:
        return "BADF_UNDECODED"
    if (lo >> 8) == 0x20:
        return "BADF_FLOORSWEPT"
    return "BADF"


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


def rrow(bar, rows, plm_suffix="PLM"):
    for name, off in rows:
        v = bar.rd(off)
        note = f" ({decode_plm(v)})" if name.endswith(plm_suffix) else ""
        b = badf_bucket(v)
        tag = f" [{b}]" if b else ""
        print(f"  {name:<22} = {v:#010x}{note}{tag}")


def read_rebar(devpath):
    """ReBAR ECAM probe via sysfs config (ssdiag 0.5.x parity, loader
    pci.rs decode semantics): walk the extended-capability chain for cap
    id 0x15, then decode ctrl/cap - size_idx n = 2^n MiB, supported size
    = 2^(bit-4) MiB for set bits >= 4 in the capability dword."""
    try:
        with open(os.path.join(devpath, "config"), "rb") as f:
            cfg = f.read(4096)
    except OSError as e:
        print(f"  ReBAR: config space unreadable ({e})")
        return
    if len(cfg) < 0x100:
        return
    off = 0x100
    seen = 0
    while 0x100 <= off <= len(cfg) - 4 and seen < 48:
        cap_id = int.from_bytes(cfg[off : off + 2], "little")
        nxt = int.from_bytes(cfg[off + 2 : off + 4], "little") >> 4
        if cap_id == 0x15:  # PCI_EXT_CAP_ID_REBAR
            for bar_idx in range(8):
                ctrl = int.from_bytes(cfg[off + 4 + bar_idx * 4 : off + 8 + bar_idx * 4], "little") & 0x3FFF
                capd = int.from_bytes(cfg[off + 4 + 32 + bar_idx * 4 : off + 8 + 32 + bar_idx * 4], "little")
                idx = ctrl & 0xF
                sizes = [f"{1 << (b - 4)}G" if b >= 4 else f"{1 << (b + 10)}M"
                         for b in range(32) if capd & (1 << b) and b >= 4]
                if idx or capd:
                    cur = f"{1 << idx}MiB" if idx else "disabled"
                    print(f"  ReBAR BAR{bar_idx}: supported [{'/'.join(sizes) or '-'}]"
                          f" current {cur} (size_idx {idx}, ctrl {ctrl:#06x}, cap {capd:#09x})")
            return
        if nxt == 0:
            break
        off = nxt
        seen += 1
    print("  ReBAR: no resizable-BAR extended capability (0x15) in the ECAM chain")


def report(bar, tag="", devpath=None):
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
    blackwell = fam.startswith("blackwell")
    print(f"  FEAT_PLM  = {plm:#010x} ({decode_plm(plm)})")
    print(f"  FEAT_SS0  = {ss0:#010x} [{decode_ss(ss0, 8)}]")
    print(f"  FEAT_SS1  = {ss1:#010x} [{decode_ss(ss1, 1)}]")
    if "readout" in regs:
        ro = bar.rd(regs["readout"])
        note = "PLM-blocked" if (ro & 0xFFFF0000) == 0xBADF0000 else ("full speed" if ro == 0 else "")
        if blackwell and ro and note != "PLM-blocked":
            note = f"fields {decode_readout1(ro)} - not rates"
        print(f"  READOUT_1 = {ro:#010x} {note}")

    def rd_tagged(name, off):
        v = bar.rd(off)
        b = badf_bucket(v)
        return v, (f" [{b}]" if b else "")

    print("  --- core/debug block (ssdiag 0.6.4 surface) ---")
    for name, off in CORE_ROWS:
        v, t = rd_tagged(name, off)
        print(f"  {name:<22} = {v:#010x}{t}")
    print("  --- privileged (badf at PL0 is the norm, still data) ---")
    rrow(bar, PRIV_ROWS)
    print("  --- family-specific / survey (badf on the wrong die is expected) ---")
    rrow(bar, ADA_ROWS)
    rrow(bar, AD1X_ROWS)
    if blackwell:
        print("  --- blackwell fuse/aux block (read-only; selectors are fuse mirrors) ---")
        rrow(bar, BLACKWELL_ROWS)
        print(f"  READOUT_1 fields (imla0 fmla16 fmla32 ffma imla1 imla2 imla3 imla4 DP) ="
              f" {decode_readout1(bar.rd(0x0082_3818))}")
        print("  note: on Blackwell selector 3 measures ~HALF rate, not the Ada 1/8 - trust the raw word")
        print("  note: FUSE_EN_SW_OVERRIDE=0 gates CTRL_OPT floorsweep only; FUSE_FEAT_OVR_DIS=0 leaves the"
              " override path open; FUSE_SS_PLM is NOT the effective gate")
    if fam in ("ampere", "ada"):
        rrow(bar, ADA_FUSE_ROWS)
        rrow(bar, OPT_FUSE_ROWS)
        print("  --- GA100-style per-GPC SM banks (badf elsewhere = absent) ---")
        ovr = "  ".join(f"{bar.rd(GA100_SM_OVR_BASE + 4 * g):#010x}" for g in range(GA100_GPCS))
        stat = "  ".join(f"{bar.rd(GA100_SM_STATUS_BASE + 4 * g):#010x}" for g in range(GA100_GPCS))
        print(f"  GA100_SM_OVR  {ovr}")
        print(f"  GA100_SM_STAT {stat}")
        print("  --- PCIe link block (XVE shadows; consumer dies read stock/benign) ---")
        rrow(bar, PCIE_LINK_ROWS)
        print("  --- XP3G fuse-override block (Gen2 capability layer) ---")
        rrow(bar, XP3G_ROWS)
    for i in range(0, len(PLM_SURVEY), 4):
        row = PLM_SURVEY[i:i+4]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    print("  --- ampere/ada FEAT sweep ---")
    for i in range(0, len(AMPADA_SWEEP), 5):
        row = AMPADA_SWEEP[i:i+5]
        print("  " + "  ".join(f"{o:#09x}={bar.rd(o):#010x}" for o in row))
    can = canary_state(bar)
    print(f"  CANARY_A/B = {bar.rd(CANARY_A):#010x}/{bar.rd(CANARY_B):#010x} -> {can}")
    unlocked = ss0 == SS0_UNLOCK and ss1 == SS1_UNLOCK

    print(f"  VERDICT: {'UNLOCKED (override values live)' if unlocked else 'not unlocked'}"
          f"{' - reapply available (door open)' if not unlocked and plm == 0xFFFF_FFFF else ''}")
    if blackwell:
        print("  FAMILY NOTE: no SEC2 stage and no GB20x booter on Blackwell - the transaction cannot run"
              " on this die; the FEAT block is read-only here (docs/blackwell-gb20x-status.md)")
        print("  FAMILY NOTE: nonzero SS0/SS1 are pre-driver shadow state, not an unlock -"
              " enable bits clear means the fused selectors are in force")
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
        st = report(bar, devpath=devpath)
        read_rebar(devpath)
        if st is None:
            continue
        fam, regs, plm, unlocked = st
        if args.reapply:
            if plm != 0xFFFF_FFFF:
                if fam.startswith("blackwell"):
                    print("  reapply REFUSED: the PLM gate only opens from PL3 firmware and no GB20x payload"
                          " exists - nothing to re-apply, the EFI chain cannot open it either")
                else:
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
