# SPDX-License-Identifier: GPL-2.0-only

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


TOOL = Path(__file__).resolve().parents[1] / "collect_compat.py"


class CollectCompatTests(unittest.TestCase):
    def write_log(self, lines, encoding):
        with tempfile.NamedTemporaryFile(mode="w", encoding=encoding, delete=False) as stream:
            log = Path(stream.name)
            stream.write("\n".join(lines) + "\n")
        self.addCleanup(log.unlink, missing_ok=True)
        return log

    def profile(self, lines, *, encoding="utf-8", additional_logs=()):
        logs = [self.write_log(block, encoding) for block in (lines, *additional_logs)]
        output = logs[0].with_suffix(".json")
        self.addCleanup(output.unlink, missing_ok=True)
        command = [sys.executable, "-B", str(TOOL), "--out", str(output)]
        for log in logs:
            command.extend(["--log", str(log)])
        subprocess.run(command, check=True, capture_output=True, text=True, encoding="utf-8")
        return json.loads(output.read_text(encoding="utf-8"))

    def test_nvidia_pci_header_keeps_valid_identity(self):
        profile = self.profile([r"===== GPU 1/1: PCI\VEN_10DE&DEV_28A0&SUBSYS_000110DE ====="])
        self.assertEqual([g["device_id"] for g in profile["gpus"]], ["10de:28a0"])

    def test_windows_pci_header_does_not_relabel_other_vendors(self):
        profile = self.profile([r"===== GPU 1/1: PCI\VEN_8086&DEV_1234&SUBSYS_00018086 ====="])
        self.assertEqual([g["device_id"] for g in profile["gpus"]], ["unknown"])

    def test_enumerated_non_display_function_overrides_fallback_hint(self):
        profile = self.profile([
            "PCI 0000:03:00.0 10de:28a0 class 000300",
            "SAMPLE: GPU 10de:22c1 @ 0000:03:00.1",
            "PCI 0000:03:00.1 10de:22c1 class 000403",
            "SAMPLE: GPU 10de:22c1 @ 0000:03:00.1",
            "PCI 0000:03:00.2 10de:22c2 class 000c03",
            "SAMPLE: GPU 10de:22c2 @ 0000:03:00.2",
        ])
        self.assertEqual([g["device_id"] for g in profile["gpus"]], ["10de:28a0"])

    def test_non_display_class_takes_precedence_across_logs(self):
        enumeration = [
            "PCI 0000:03:00.0 10de:28a0 class 000300",
            "PCI 0000:03:00.1 10de:22c1 class 000403",
        ]
        hints = [
            "GPU 1: Synthetic secondary function",
            "GPU 1 PCI: 10de:22c1 BDF 03:00.1",
            "SAMPLE: GPU 10de:22c1 @ 0000:03:00.1",
        ]
        for first, second in ((enumeration, hints), (hints, enumeration)):
            profile = self.profile(first, additional_logs=(second,))
            self.assertEqual([g["device_id"] for g in profile["gpus"]], ["10de:28a0"])

    def test_non_display_class_does_not_hide_a_different_bdf_with_the_same_id(self):
        enumeration = ["PCI 0000:03:00.1 10de:28a0 class 000403"]
        audio_inventory = ["GPU 1: Synthetic audio", "GPU 1 PCI: 10de:28a0 BDF 03:00.1"]
        for hints, expected_bdf, expected_desc in (
            (["SAMPLE: GPU 10de:28a0 @ 0000:04:00.0"], "", ""),
            (["GPU 1: Synthetic display", "GPU 1 PCI: 10de:28a0 BDF 04:00.0"],
             "04:00.0", "Synthetic display"),
        ):
            with self.subTest(hints=hints):
                profile = self.profile(audio_inventory, additional_logs=(hints, enumeration))
                gpu, = profile["gpus"]
                self.assertEqual((gpu["device_id"], gpu["bdf"], gpu["desc"]),
                                 ("10de:28a0", expected_bdf, expected_desc))

    def test_malformed_windows_pci_id_cannot_supply_a_four_digit_prefix(self):
        profile = self.profile([r"===== GPU 1/1: PCI\VEN_10DE&DEV_28A0F ====="])
        self.assertEqual([g["device_id"] for g in profile["gpus"]], ["unknown"])

    def test_malformed_inventory_id_cannot_supply_a_four_digit_prefix(self):
        profile = self.profile(["GPU 1 PCI: 10de:28a0z"])
        self.assertEqual([g["device_id"] for g in profile["gpus"]], ["unknown"])

    def test_malformed_optional_inventory_values_stay_empty(self):
        profile = self.profile([
            "GPU 1 PCI: 10de:28a0 SUBSYS 000110DE0",
            "GPU 2 PCI: 10de:28a1 BDF 03:00.0x",
        ])
        first, second = profile["gpus"]
        self.assertEqual(first["subsys"], "")
        self.assertEqual(second["bdf"], "")

    def test_malformed_sample_cannot_supply_a_valid_value(self):
        profile = self.profile([
            "SAMPLE FEAT_OVR_PLM = 0xffffff8f [ok]",
            "SAMPLE FEAT_OVR_PLM = 0xffffffffjunk [ok]",
            "SAMPLE SEC2_CPUCTL = 0x1badf5620 [BADF_VARIANT]",
        ])
        self.assertEqual(profile["gpus"][0]["fields"],
                         {"FEAT_OVR_PLM": {"value": 0xffffff8f, "bucket": "ok"}})

    def test_inventory_and_readonly_completion_are_observations(self):
        profile = self.profile([
            "GPU 1: Synthetic NVIDIA adapter",
            "GPU 1 PCI: 10de:28a0 SUBSYS 000110DE BDF 03:00.0",
            "GPU 1 family: ad102 (Ada) - IN-SCOPE",
            "GPU 1 driver: 621.24",
            "PHASE: SAMPLE (M1 read-only cold state)",
            "SAMPLE FEAT_OVR_PLM 0x823804 = 0xffffff8f [ok]",
            "SAMPLE SEC2_CPUCTL 0x840100 = 0xbadf5620 [BADF_VARIANT]",
            "SAMPLE: FEAT sweep (dword stride): 2 ok / 0 badf",
            "FINAL: read-only run complete; nothing was written to the GPU.",
        ], encoding="utf-16")
        gpu, = profile["gpus"]
        self.assertEqual((gpu["device_id"], gpu["subsys"], gpu["bdf"]),
                         ("10de:28a0", "000110DE", "03:00.0"))
        self.assertEqual(gpu["family"], "ad102 (Ada)")
        self.assertEqual(gpu["driver"], "621.24")
        self.assertNotIn("driver_mode", gpu)
        self.assertEqual(gpu["fields"]["FEAT_OVR_PLM 0x823804"]["value"], 0xffffff8f)
        self.assertEqual(gpu["fields"]["SEC2_CPUCTL 0x840100"]["bucket"], "BADF_VARIANT")
        self.assertEqual(profile["loader_verdicts"],
                         ["FINAL: read-only run complete; nothing was written to the GPU."])

    def test_same_boot_loader_sample_and_ssdiag_equals_field(self):
        loader = ["SAMPLE FEAT_OVR_ECC 0x82380c = 0x00000001 [ok]"]
        ssdiag = ["WPR2_LO                = 0x00002000"]
        for first, second in ((loader, ssdiag), (ssdiag, loader)):
            with self.subTest(first=first):
                gpu, = self.profile(first, additional_logs=(second,))["gpus"]
                self.assertEqual(gpu["fields"], {
                    "FEAT_OVR_ECC 0x82380c": {"value": 1, "bucket": "ok"},
                    "WPR2_LO": {"value": 0x2000, "bucket": ""},
                })

    def test_ssdiag_offset_field_uses_recorded_value(self):
        gpu, = self.profile(["WPR2_LO  0x001fa824= 0x1ffffe00"])["gpus"]
        self.assertEqual(gpu["fields"]["WPR2_LO"]["value"], 0x1ffffe00)

    def test_ssdiag_offset_without_valid_value_is_ignored(self):
        for rhs in ("", "unavailable", "0x123456789", "0x12junk"):
            with self.subTest(rhs=rhs):
                gpu, = self.profile([f"WPR2_LO  0x001fa824= {rhs}"])["gpus"]
                self.assertNotIn("WPR2_LO", gpu["fields"])

    def test_two_device_sections_keep_their_own_fields(self):
        profile = self.profile([
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
            r"===== GPU 2/2: PCI\VEN_10DE&DEV_28E0&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000022",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(by_id["10de:2504"]["fields"]["WPR2_LO"]["value"], 0x11)
        self.assertEqual(by_id["10de:28e0"]["fields"]["WPR2_LO"]["value"], 0x22)

    def test_section_field_attribution_is_order_independent(self):
        profile = self.profile([
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_28E0&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000022",
            r"===== GPU 2/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(by_id["10de:2504"]["fields"]["WPR2_LO"]["value"], 0x11)
        self.assertEqual(by_id["10de:28e0"]["fields"]["WPR2_LO"]["value"], 0x22)

    def test_section_without_fields_gets_no_neighbors_fields(self):
        profile = self.profile([
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
            r"===== GPU 2/2: PCI\VEN_10DE&DEV_28E0&SUBSYS_000110DE =====",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(by_id["10de:28e0"]["fields"], {})

    def test_disjoint_section_field_names_do_not_cross_contaminate(self):
        profile = self.profile([
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
            r"===== GPU 2/2: PCI\VEN_10DE&DEV_28E0&SUBSYS_000110DE =====",
            "  FEAT_OVR_PLM           0x00823804= 0xffffff8f",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(set(by_id["10de:2504"]["fields"]), {"WPR2_LO"})
        self.assertEqual(set(by_id["10de:28e0"]["fields"]), {"FEAT_OVR_PLM"})

    def test_headerless_fields_stay_shared_and_attributed_fields_win(self):
        profile = self.profile([
            "  WPR2_LO                0x001fa824= 0x00000033",
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
            r"===== GPU 2/2: PCI\VEN_10DE&DEV_28E0&SUBSYS_000110DE =====",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(by_id["10de:2504"]["fields"]["WPR2_LO"]["value"], 0x11)
        self.assertEqual(by_id["10de:28e0"]["fields"]["WPR2_LO"]["value"], 0x33)

    def test_multi_device_header_keeps_previous_section(self):
        profile = self.profile([
            r"===== GPU 1/2: PCI\VEN_10DE&DEV_2504&SUBSYS_000110DE =====",
            "  WPR2_LO                0x001fa824= 0x00000011",
            "FOUND target PCI\\VEN_10DE&DEV_2504 and PCI\\VEN_10DE&DEV_28E0",
            "  FEAT_OVR_PLM           0x00823804= 0xffffff8f",
        ])
        by_id = {g["device_id"]: g for g in profile["gpus"]}
        self.assertEqual(by_id["10de:2504"]["fields"]["WPR2_LO"]["value"], 0x11)
        self.assertEqual(by_id["10de:2504"]["fields"]["FEAT_OVR_PLM"]["value"],
                         0xffffff8f)
        self.assertEqual(by_id["10de:28e0"]["fields"], {})

    def test_empty_input_stays_unknown(self):
        profile = self.profile(["SAMPLE FEAT_OVR_PLM = not-a-value"])
        gpu, = profile["gpus"]
        self.assertEqual((gpu["device_id"], gpu["bdf"], gpu["driver"]), ("unknown", "", ""))
        self.assertEqual(gpu["fields"], {})
        self.assertEqual(profile["loader_verdicts"], [])


if __name__ == "__main__":
    unittest.main()
