# SPDX-License-Identifier: GPL-2.0-only

import importlib.util
from pathlib import Path
import struct
import subprocess
import sys
import unittest


TOOL_PATH = Path(__file__).resolve().parents[1] / "diag-unlock.py"


def load_diag_unlock():
    spec = importlib.util.spec_from_file_location("diag_unlock", TOOL_PATH)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def write_pci_rom(buf: bytearray, off: int, tag: bytes, code_type: int, blocks_512: int):
    buf[off : off + 2] = b"\x55\xaa"
    struct.pack_into("<H", buf, off + 0x18, 0x20)
    p = off + 0x20
    buf[p : p + 4] = tag
    struct.pack_into("<H", buf, p + 0x0A, 0x18)
    struct.pack_into("<H", buf, p + 0x10, blocks_512)
    buf[p + 0x14] = code_type


def write_bit(buf: bytearray, bit_off: int, token_data_raw_ptr: int):
    struct.pack_into("<H", buf, bit_off, 0xB8FF)
    buf[bit_off + 2 : bit_off + 6] = b"BIT\x00"
    struct.pack_into("<H", buf, bit_off + 6, 0x0100)
    buf[bit_off + 8] = 12  # hdr_size
    buf[bit_off + 9] = 6   # stride
    buf[bit_off + 10] = 1  # token_count
    buf[bit_off + 11] = (256 - (sum(buf[bit_off : bit_off + 11]) & 0xFF)) & 0xFF
    tok = bit_off + 12
    buf[tok] = 0x70
    buf[tok + 1] = 0x02
    struct.pack_into("<H", buf, tok + 2, 4)
    struct.pack_into("<H", buf, tok + 4, token_data_raw_ptr)


class DiagUnlockPromBitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mod = load_diag_unlock()

    def test_blackwell_gb205_family_maps_to_823_bank(self):
        for boot0 in (0x1B5_000A1, 0x1B6_000A1, 0x1A0_000A1, 0x2112_0000):
            fam, regs = self.mod.family_of(boot0)
            self.assertTrue(fam.startswith("blackwell"))
            self.assertEqual(regs["plm"], 0x0082_3804)
            self.assertEqual(regs["readout"], 0x0082_3818)

    def test_prom_bit_blackwell_v6_with_ff_padding_bogus_55aa_null_slots_and_oob(self):
        rom = bytearray(b"\xff" * 0x4000)
        # Bogus 0x55AA at 0x400 with PCIR ptr = 0 (like GB205 0x33c00)
        rom[0x400:0x402] = b"\x55\xaa"
        struct.pack_into("<H", rom, 0x418, 0)

        write_pci_rom(rom, 0x600, b"PCIR", 0x00, 2)
        write_pci_rom(rom, 0xA00, b"PCIR", 0x03, 2)
        write_pci_rom(rom, 0xE00, b"NPDS", 0xE0, 4)

        write_bit(rom, 0x680, 0x0100)
        struct.pack_into("<I", rom, 0x700, 0x0600)

        tbl = 0x1000
        rom[tbl : tbl + 4] = bytes([1, 6, 6, 4])
        rom[tbl + 6 : tbl + 12] = b"\x00" * 6  # empty slot 0
        rom[tbl + 12] = 0x23
        rom[tbl + 13] = 0x0E
        struct.pack_into("<I", rom, tbl + 14, 0x1000)
        rom[tbl + 18 : tbl + 24] = b"\x00" * 6  # empty slot 2
        rom[tbl + 24] = 0x24
        rom[tbl + 25] = 0x0F
        struct.pack_into("<I", rom, tbl + 26, 0x5000)  # OOB

        struct.pack_into("<I", rom, 0x1A00, (80 << 16) | (6 << 8) | 0x1F)
        struct.pack_into("<I", rom, 0x1A04, 0x0002_7000)

        rep = self.mod.parse_prom_bit(bytes(rom))
        self.assertIsNone(rep["error"])
        self.assertEqual(len(rep["rom_images"]), 3)
        self.assertEqual(rep["image_base"], 0x600)
        self.assertEqual(rep["expansion_off"], 0x400)
        self.assertEqual(rep["adjust"], 0xA00)
        tbl_rep, = rep["ucode_tables"]
        self.assertEqual(tbl_rep["slot_count"], 4)
        self.assertEqual(tbl_rep["empty_slots"], 2)
        self.assertEqual(len(tbl_rep["entries"]), 2)
        e0, e1 = tbl_rep["entries"]
        self.assertEqual(e0["kind"], "readable")
        self.assertEqual((e0["app_id"], e0["target_id"]), (0x23, 0x0E))
        self.assertEqual((e0["desc_ver"], e0["desc_size"], e0["encrypted"]), (6, 80, True))
        self.assertEqual(e0["stored_size"], 0x27000)
        self.assertEqual(e0["meta"], {"kind": "v6", "image_start": 0x1A50})
        self.assertEqual(e1, {
            "kind": "oob",
            "app_id": 0x24,
            "target_id": 0x0F,
            "desc_rel": 0x5000,
            "abs_off": 0x5A00,
        })

    def test_prom_bit_v2_and_v3_descriptors(self):
        rom = bytearray(0x2000)
        write_pci_rom(rom, 0x000, b"PCIR", 0x00, 4)
        write_bit(rom, 0x100, 0x0200)
        struct.pack_into("<I", rom, 0x200, 0x0300)
        tbl = 0x300
        rom[tbl : tbl + 4] = bytes([1, 6, 6, 2])
        # Slot 0: V2 plaintext
        rom[tbl + 6] = 0x85
        rom[tbl + 7] = 0x01
        struct.pack_into("<I", rom, tbl + 8, 0x400)
        struct.pack_into("<I", rom, 0x400, (60 << 16) | (2 << 8))
        struct.pack_into("<I", rom, 0x404, 0x8000)
        struct.pack_into("<I", rom, 0x400 + 24, 0x1200)
        struct.pack_into("<I", rom, 0x400 + 32, 0x400)
        struct.pack_into("<I", rom, 0x400 + 36, 0xE00)
        struct.pack_into("<I", rom, 0x400 + 48, 0x600)
        # Slot 1: V3 encrypted
        rom[tbl + 12] = 0x45
        rom[tbl + 13] = 0x01
        struct.pack_into("<I", rom, tbl + 14, 0x500)
        struct.pack_into("<I", rom, 0x500, (44 << 16) | (3 << 8) | 0x04)
        struct.pack_into("<I", rom, 0x504, 0x9000)
        struct.pack_into("<I", rom, 0x508, 0x200)
        struct.pack_into("<H", rom, 0x500 + 36, 0x0400)
        rom[0x500 + 38] = 5
        rom[0x500 + 39] = 2
        struct.pack_into("<H", rom, 0x500 + 40, 0x0003)

        rep = self.mod.parse_prom_bit(bytes(rom))
        self.assertIsNone(rep["error"])
        e0, e1 = rep["ucode_tables"][0]["entries"]
        self.assertEqual((e0["desc_ver"], e0["encrypted"]), (2, False))
        self.assertEqual(e0["meta"]["imem_load"], 0x1200)
        self.assertEqual((e1["desc_ver"], e1["encrypted"]), (3, True))
        self.assertEqual(e1["meta"]["sig_count"], 2)

    def test_prom_bit_rejects_unreadable_and_bad_checksum(self):
        self.assertEqual(self.mod.parse_prom_bit(b"\x00" * 64)["error"], "unreadable")
        self.assertEqual(self.mod.parse_prom_bit(b"\xff" * 64)["error"], "unreadable")
        badf = bytearray(64)
        struct.pack_into("<I", badf, 0, 0xBADF_5040)
        self.assertEqual(self.mod.parse_prom_bit(bytes(badf))["error"], "unreadable")

        bad = bytearray(0x200)
        write_bit(bad, 0x40, 0x80)
        bad[0x40 + 11] ^= 0x01
        self.assertEqual(self.mod.parse_prom_bit(bytes(bad))["error"], "no_bit_header")

    def test_cli_rejects_prom_bit_with_write_flags(self):
        for write_flag in ("--reapply", "--fuserows=0-3"):
            proc = subprocess.run(
                [sys.executable, "-B", str(TOOL_PATH), "--prom-bit", write_flag],
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc.returncode, 2)
            self.assertIn("read-only", proc.stderr)


if __name__ == "__main__":
    unittest.main()
