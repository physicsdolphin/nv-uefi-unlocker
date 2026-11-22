# SPDX-License-Identifier: GPL-2.0-only

import importlib.util
from pathlib import Path
import unittest


TOOL_PATH = Path(__file__).resolve().parents[1] / "bench" / "blackwell_clamp_eval.py"


def load_eval_module():
    spec = importlib.util.spec_from_file_location("blackwell_clamp_eval", TOOL_PATH)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class BlackwellClampEvalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mod = load_eval_module()

    def test_arch_key_mapping(self):
        self.assertEqual(self.mod.arch_key(12, 0), "blackwell_gb20x")
        self.assertEqual(self.mod.arch_key(8, 9), "ada")
        self.assertEqual(self.mod.arch_key(8, 6), "ampere_ga10x")
        self.assertEqual(self.mod.arch_key(7, 5), "turing")
        self.assertIsNone(self.mod.arch_key(6, 1))

    def test_flop_per_clk_sm_math(self):
        # 46 SMs @ 2000 MHz, 47.104 TFLOPS => 512.0 FLOP/clk/SM
        got = self.mod.flop_per_clk_sm(47.104, 2000.0, 46)
        self.assertAlmostEqual(got, 512.0, places=3)
        self.assertIsNone(self.mod.flop_per_clk_sm(50.0, 0.0, 46))
        self.assertIsNone(self.mod.flop_per_clk_sm(50.0, None, 46))

    def test_classify_case_boundaries(self):
        self.assertIn("CLAMPED", self.mod.classify_case("fp16_fp32acc", 0.51))
        self.assertIn("near full", self.mod.classify_case("bf16_fp32acc", 0.92))
        self.assertIn("intermediate", self.mod.classify_case("tf32_fp32acc", 0.72))
        self.assertIn("SIMT control", self.mod.classify_case("fp32_control", None))
        self.assertIn("informational", self.mod.classify_case("int8", 0.30))

    def test_telemetry_summary_filters_idle_ramp_and_tail_clocks(self):
        # Simulates the exact FP8 bug from short-burst polling: initial ramp +
        # tail idle 247 MHz sample must not drag down steady-state 2300 MHz clock.
        samples = [
            {"t": 0.00, "sm_clock_mhz": 247.0, "power_w": 18.0, "temperature_c": 52.0, "utilization_gpu_pct": 10.0, "throttle_reasons": 1},
            {"t": 0.10, "sm_clock_mhz": 1100.0, "power_w": 45.0, "temperature_c": 55.0, "utilization_gpu_pct": 60.0, "throttle_reasons": 4},
            {"t": 0.30, "sm_clock_mhz": 2280.0, "power_w": 96.0, "temperature_c": 60.0, "utilization_gpu_pct": 99.0, "throttle_reasons": 4},
            {"t": 0.40, "sm_clock_mhz": 2310.0, "power_w": 98.0, "temperature_c": 61.0, "utilization_gpu_pct": 100.0, "throttle_reasons": 4},
            {"t": 0.50, "sm_clock_mhz": 2300.0, "power_w": 99.0, "temperature_c": 62.0, "utilization_gpu_pct": 100.0, "throttle_reasons": 4},
            {"t": 0.60, "sm_clock_mhz": 247.0, "power_w": 22.0, "temperature_c": 59.0, "utilization_gpu_pct": 5.0, "throttle_reasons": 1},
        ]
        summary = self.mod.summarize_telemetry_samples(samples, settle_s=0.25)
        self.assertEqual(summary["sm_clock_mhz"], 2300.0)
        self.assertEqual(summary["power_w"], 98.0)
        self.assertEqual(summary["power_peak_w"], 99.0)
        self.assertEqual(summary["temperature_peak_c"], 62.0)
        self.assertEqual(summary["telemetry_active_samples"], 3)


if __name__ == "__main__":
    unittest.main()
