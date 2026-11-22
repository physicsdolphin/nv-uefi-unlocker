#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Cross-platform GeForce / Blackwell (SM 12.0) tensor-clamp evaluator.

Runs supported userspace PyTorch CUDA GEMMs and samples live SM clocks, power,
temperature, and throttle reasons (NVML with nvidia-smi fallback) across
duration-bounded steady-state bursts. Designed to work identically on Linux and
Windows across PyTorch 2.x builds without kernel drivers or BAR0 access.

Why clock-normalized FLOP/clk/SM matters:
  On power-limited laptops (and power-capped desktops), raw TFLOP/s depends on
  boost clock. Dividing achieved throughput by (SM_count * active_SM_clock)
  isolates the hardware tensor issue-rate clamp (~0.5x of architectural full
  rate on FP32-accumulate tensor paths, while FP16-accumulate and FP32 SIMT
  remain unclamped).

Usage:
  python tools/bench/blackwell_clamp_eval.py --n 8192 --min-seconds 2.0 --json eval.json
  python tools/bench/blackwell_clamp_eval.py --device 0 --no-paired-fp16
"""

import argparse
import json
import platform
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

# Architectural dense tensor ceilings (unclamped full rate, FLOP/clk/SM = 2*MACs).
# Sources: NVIDIA Turing / Ampere / Ada / Blackwell whitepapers & docs/theoretical-uplift.md.
FULL_RATE_FLOP_CLK_SM = {
    "blackwell_gb20x": {  # SM 12.0 (RTX 50 consumer)
        "fp16_fp32acc": 1024,
        "bf16_fp32acc": 1024,
        "tf32_fp32acc": 512,
        "fp8_fp32acc": 2048,
        "fp16_fp16acc": 1024,
        "int8": 2048,
    },
    "ada": {  # SM 8.9 (RTX 40)
        "fp16_fp32acc": 1024,
        "bf16_fp32acc": 1024,
        "tf32_fp32acc": 512,
        "fp8_fp32acc": 2048,
        "fp16_fp16acc": 1024,
        "int8": 2048,
    },
    "ampere_ga10x": {  # SM 8.6 (RTX 30)
        "fp16_fp32acc": 1024,
        "bf16_fp32acc": 1024,
        "tf32_fp32acc": 512,
        "fp8_fp32acc": 0,
        "fp16_fp16acc": 1024,
        "int8": 2048,
    },
    "turing": {  # SM 7.5 (RTX 20)
        "fp16_fp32acc": 512,
        "bf16_fp32acc": 0,
        "tf32_fp32acc": 0,
        "fp8_fp32acc": 0,
        "fp16_fp16acc": 1024,
        "int8": 2048,
    },
}

CLAMP_CASES = {"fp16_fp32acc", "bf16_fp32acc", "tf32_fp32acc", "fp8_fp32acc"}


def arch_key(cc_major, cc_minor):
    if cc_major == 12:
        return "blackwell_gb20x"
    if cc_major == 8 and cc_minor == 9:
        return "ada"
    if cc_major == 8 and cc_minor >= 6:
        return "ampere_ga10x"
    if cc_major == 7 and cc_minor == 5:
        return "turing"
    return None


def flop_per_clk_sm(tflops, sm_clock_mhz, sm_count):
    if not tflops or not sm_clock_mhz or not sm_count:
        return None
    if sm_clock_mhz <= 0 or sm_count <= 0:
        return None
    return tflops * 1e12 / (sm_clock_mhz * 1e6 * sm_count)


def classify_case(case, ratio):
    if case == "fp32_control":
        return "SIMT control (unaffected by tensor clamp)"
    if case == "fp16_fp16acc":
        return "low-precision-accumulate control (unclamped path)"
    if case == "int8":
        return "informational (PyTorch kernel/shape dependent)"
    if ratio is None:
        return "no architectural reference or missing clock"
    if case in CLAMP_CASES:
        if ratio <= 0.65:
            return "CLAMPED (~half of full tensor rate)"
        if ratio >= 0.80:
            return "near full tensor rate"
        return "intermediate (check power/clock limits)"
    return ""


def summarize_telemetry_samples(samples, settle_s=0.25):
    """Aggregate steady-state telemetry samples, filtering idle ramp/tail valleys."""
    if not samples:
        return {
            "sm_clock_mhz": None,
            "sm_clock_peak_mhz": None,
            "power_w": None,
            "power_peak_w": None,
            "temperature_peak_c": None,
            "utilization_gpu_median_pct": None,
            "throttle_reasons_mask": None,
            "telemetry_samples": 0,
            "telemetry_active_samples": 0,
        }

    t0 = samples[0].get("t", 0.0)
    settled = [row for row in samples if row.get("t", t0) - t0 >= settle_s]
    pool = settled if len(settled) >= 3 else samples

    all_clocks = [r["sm_clock_mhz"] for r in pool if r.get("sm_clock_mhz") is not None]
    peak_clock = max(all_clocks) if all_clocks else 0.0
    all_powers = [r["power_w"] for r in pool if r.get("power_w") is not None]
    peak_power = max(all_powers) if all_powers else 0.0
    power_floor = max(20.0, peak_power * 0.50)

    active = [
        r
        for r in pool
        if (r.get("utilization_gpu_pct", 0.0) >= 50.0 or r.get("power_w", 0.0) >= power_floor)
        and (peak_clock <= 0.0 or r.get("sm_clock_mhz", peak_clock) >= peak_clock * 0.50)
    ]
    selected = active or pool

    def vals(key):
        return [r[key] for r in selected if r.get(key) is not None]

    clocks = vals("sm_clock_mhz")
    powers = vals("power_w")
    temps = vals("temperature_c")
    utils = vals("utilization_gpu_pct")
    reasons = vals("throttle_reasons")
    reason_mask = 0
    for m in reasons:
        reason_mask |= int(m)

    return {
        "sm_clock_mhz": statistics.median(clocks) if clocks else None,
        "sm_clock_peak_mhz": max(clocks) if clocks else None,
        "power_w": statistics.median(powers) if powers else None,
        "power_peak_w": max(powers) if powers else None,
        "temperature_peak_c": max(temps) if temps else None,
        "utilization_gpu_median_pct": statistics.median(utils) if utils else None,
        "throttle_reasons_mask": reason_mask if reasons else None,
        "telemetry_samples": len(samples),
        "telemetry_active_samples": len(active),
    }


def decode_throttle_reasons(nv, mask):
    if nv is None or mask is None:
        return []
    names = [
        ("gpu_idle", "nvmlClocksThrottleReasonGpuIdle"),
        ("applications_clocks", "nvmlClocksThrottleReasonApplicationsClocksSetting"),
        ("software_power_cap", "nvmlClocksThrottleReasonSwPowerCap"),
        ("hardware_slowdown", "nvmlClocksThrottleReasonHwSlowdown"),
        ("sync_boost", "nvmlClocksThrottleReasonSyncBoost"),
        ("software_thermal", "nvmlClocksThrottleReasonSwThermalSlowdown"),
        ("display_clock", "nvmlClocksThrottleReasonDisplayClockSetting"),
        ("hardware_power_brake", "nvmlClocksThrottleReasonHwPowerBrakeSlowdown"),
        ("hardware_thermal", "nvmlClocksThrottleReasonHwThermalSlowdown"),
    ]
    out = []
    for label, attr in names:
        bit = getattr(nv, attr, 0)
        if bit and (mask & bit):
            out.append(label)
    return out


def query_nvidia_smi_meta(device_index):
    try:
        out = subprocess.check_output(
            [
                "nvidia-smi",
                "-i",
                str(device_index),
                "--query-gpu=driver_version,vbios_version,pci.device_id,power.limit,power.max_limit",
                "--format=csv,noheader",
            ],
            text=True,
            timeout=5,
            stderr=subprocess.DEVNULL,
        ).strip()
        parts = [p.strip() for p in out.split(",")]
        if len(parts) >= 5:
            return {
                "driver": parts[0],
                "vbios": parts[1],
                "pci_device_id": parts[2],
                "power_limit": parts[3],
                "power_max_limit": parts[4],
            }
    except (OSError, subprocess.SubprocessError):
        pass
    return {}


def nvml_handle(device_index):
    try:
        import pynvml

        pynvml.nvmlInit()
        return pynvml, pynvml.nvmlDeviceGetHandleByIndex(device_index)
    except Exception:
        return None, None


class TelemetrySampler:
    """Background NVML (or nvidia-smi fallback) sampler during queued CUDA work."""

    def __init__(self, nv, handle, device_index=0, interval=None):
        self.nv = nv
        self.handle = handle
        self.device_index = device_index
        self.interval = interval if interval is not None else (0.05 if nv is not None else 0.25)
        self.samples = []
        self._stop = threading.Event()
        self._thread = None

    def _read_nvml(self):
        nv, h = self.nv, self.handle
        row = {"t": time.monotonic()}
        readers = {
            "sm_clock_mhz": lambda: float(nv.nvmlDeviceGetClockInfo(h, nv.NVML_CLOCK_SM)),
            "power_w": lambda: nv.nvmlDeviceGetPowerUsage(h) / 1000.0,
            "temperature_c": lambda: float(nv.nvmlDeviceGetTemperature(h, nv.NVML_TEMPERATURE_GPU)),
            "utilization_gpu_pct": lambda: float(nv.nvmlDeviceGetUtilizationRates(h).gpu),
            "throttle_reasons": lambda: int(nv.nvmlDeviceGetCurrentClocksThrottleReasons(h)),
        }
        for key, fn in readers.items():
            try:
                row[key] = fn()
            except Exception:
                pass
        return row if len(row) > 1 else None

    def _read_smi(self):
        try:
            out = subprocess.check_output(
                [
                    "nvidia-smi",
                    "-i",
                    str(self.device_index),
                    "--query-gpu=clocks.sm,power.draw,temperature.gpu,utilization.gpu",
                    "--format=csv,noheader,nounits",
                ],
                text=True,
                timeout=3,
                stderr=subprocess.DEVNULL,
            ).strip()
            parts = [p.strip() for p in out.split(",")]
            if len(parts) < 4:
                return None
            row = {"t": time.monotonic()}
            for key, raw in zip(
                ("sm_clock_mhz", "power_w", "temperature_c", "utilization_gpu_pct"), parts
            ):
                try:
                    row[key] = float(raw)
                except ValueError:
                    pass
            return row if len(row) > 1 else None
        except (OSError, subprocess.SubprocessError):
            return None

    def _run(self):
        while not self._stop.is_set():
            row = self._read_nvml() if (self.nv is not None and self.handle is not None) else self._read_smi()
            if row is not None:
                self.samples.append(row)
            self._stop.wait(self.interval)

    def start(self):
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        return self

    def stop(self):
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=3)

    def summary(self, settle_s=0.25):
        stats = summarize_telemetry_samples(self.samples, settle_s=settle_s)
        stats["throttle_reasons"] = decode_throttle_reasons(self.nv, stats["throttle_reasons_mask"])
        return stats


def _sample_accuracy(torch, out, reference):
    if reference is None or out is None:
        return None, None
    sample = out[: reference.size(0), : reference.size(1)].detach().cpu().double()
    finite = bool(sample.isfinite().all().item())
    if not finite:
        return False, None
    denom = reference.norm().item()
    if denom == 0.0:
        return True, 0.0
    rel_l2 = ((sample - reference).norm() / denom).item()
    return True, rel_l2


def _make_case(torch, device, n, case, seed):
    gen = torch.Generator(device=device).manual_seed(seed)
    try:
        if case in ("fp16_fp32acc", "fp16_fp16acc"):
            a = torch.randn(n, n, device=device, dtype=torch.float16, generator=gen)
            b = torch.randn(n, n, device=device, dtype=torch.float16, generator=gen)
            out = torch.empty(n, n, device=device, dtype=torch.float16)
            ref = a[:16, :].cpu().double() @ b[:, :16].cpu().double()
            return (lambda: torch.mm(a, b, out=out)), out, ref
        if case == "bf16_fp32acc":
            a = torch.randn(n, n, device=device, dtype=torch.bfloat16, generator=gen)
            b = torch.randn(n, n, device=device, dtype=torch.bfloat16, generator=gen)
            out = torch.empty(n, n, device=device, dtype=torch.bfloat16)
            ref = a[:16, :].cpu().double() @ b[:, :16].cpu().double()
            return (lambda: torch.mm(a, b, out=out)), out, ref
        if case in ("tf32_fp32acc", "fp32_control"):
            a = torch.randn(n, n, device=device, dtype=torch.float32, generator=gen)
            b = torch.randn(n, n, device=device, dtype=torch.float32, generator=gen)
            out = torch.empty(n, n, device=device, dtype=torch.float32)
            ref = a[:16, :].cpu().double() @ b[:, :16].cpu().double()
            return (lambda: torch.mm(a, b, out=out)), out, ref
        if case == "fp8_fp32acc":
            if not hasattr(torch, "float8_e4m3fn") or not hasattr(torch, "_scaled_mm"):
                return None, None, None
            fp8 = torch.float8_e4m3fn
            a = torch.randn(n, n, device=device, generator=gen).to(fp8)
            b = torch.randn(n, n, device=device, generator=gen).to(fp8).t()
            one = torch.tensor(1.0, device=device, dtype=torch.float32)
            ref = a[:16, :].float().cpu().double() @ b[:, :16].float().cpu().double()
            holder = [None]

            def op():
                holder[0] = torch._scaled_mm(a, b, scale_a=one, scale_b=one, out_dtype=torch.bfloat16)
                return holder[0]

            return op, holder, ref
        if case == "int8":
            if not hasattr(torch, "_int_mm"):
                return None, None, None
            a = torch.randint(-8, 8, (n, n), device=device, dtype=torch.int8, generator=gen)
            b = torch.randint(-8, 8, (n, n), device=device, dtype=torch.int8, generator=gen)
            holder = [None]

            def op():
                holder[0] = torch._int_mm(a, b)
                return holder[0]

            return op, holder, None
    except Exception as exc:
        print(f"  [skip {case}: {type(exc).__name__}: {exc}]", file=sys.stderr)
        return None, None, None
    return None, None, None


def _run_steady_burst(torch, op, n, warmup, min_iters, min_seconds, max_iters, nv, handle, device_index):
    # Warmup and pilot timing to size the burst for steady-state telemetry.
    for _ in range(warmup):
        op()
    torch.cuda.synchronize()

    p0 = torch.cuda.Event(enable_timing=True)
    p1 = torch.cuda.Event(enable_timing=True)
    p0.record()
    op()
    p1.record()
    torch.cuda.synchronize()
    pilot_s = max(p0.elapsed_time(p1) / 1000.0, 1e-4)

    target_iters = max(min_iters, int(min_seconds / pilot_s) + 1)
    iters = min(max_iters, target_iters)

    # Start background telemetry while continuously queuing CUDA GEMMs.
    events = [
        (torch.cuda.Event(enable_timing=True), torch.cuda.Event(enable_timing=True))
        for _ in range(iters)
    ]
    sampler = TelemetrySampler(nv, handle, device_index=device_index).start()
    for start, end in events:
        start.record()
        op()
        end.record()
    torch.cuda.synchronize()
    sampler.stop()

    ms_list = sorted(start.elapsed_time(end) for start, end in events)
    median_ms = statistics.median(ms_list)
    p10_ms = ms_list[max(0, int(0.10 * (len(ms_list) - 1)))]
    p90_ms = ms_list[min(len(ms_list) - 1, int(0.90 * (len(ms_list) - 1)))]
    tflops = (2.0 * (n ** 3)) / (median_ms * 1e9)
    stats = sampler.summary(settle_s=min(0.25, (iters * median_ms / 1000.0) * 0.25))
    return {
        "iters": iters,
        "median_ms": median_ms,
        "p10_ms": p10_ms,
        "p90_ms": p90_ms,
        "tflops": tflops,
        **stats,
    }


def bench_case(torch, device, device_index, n, case, warmup, min_iters, min_seconds, max_iters, seed, nv, handle):
    matmul = torch.backends.cuda.matmul
    cudnn = torch.backends.cudnn
    old_tf32 = matmul.allow_tf32
    old_cudnn_tf32 = cudnn.allow_tf32
    has_fp16_acc = hasattr(matmul, "allow_fp16_accumulation")
    old_fp16_acc = matmul.allow_fp16_accumulation if has_fp16_acc else None

    if case == "fp16_fp16acc" and not has_fp16_acc:
        print("  [skip fp16_fp16acc: PyTorch build lacks allow_fp16_accumulation]", file=sys.stderr)
        return None
    try:
        use_tf32 = case == "tf32_fp32acc"
        matmul.allow_tf32 = use_tf32
        cudnn.allow_tf32 = use_tf32
        if has_fp16_acc:
            matmul.allow_fp16_accumulation = case == "fp16_fp16acc"
        op, out_or_holder, ref = _make_case(torch, device, n, case, seed)
        if op is None:
            return None
        burst = _run_steady_burst(
            torch, op, n, warmup, min_iters, min_seconds, max_iters, nv, handle, device_index
        )
        out_tensor = out_or_holder[0] if isinstance(out_or_holder, list) else out_or_holder
        finite, rel_l2 = _sample_accuracy(torch, out_tensor, ref)
        return {
            "case": case,
            "n": n,
            "sample_finite": finite,
            "sample_relative_l2_error": rel_l2,
            **burst,
        }
    except Exception as exc:
        print(f"  [skip {case}: {type(exc).__name__}: {exc}]", file=sys.stderr)
        return None
    finally:
        matmul.allow_tf32 = old_tf32
        cudnn.allow_tf32 = old_cudnn_tf32
        if has_fp16_acc:
            matmul.allow_fp16_accumulation = old_fp16_acc


def run_paired_fp16_ab(torch, device, device_index, n, warmup, min_iters, min_seconds, max_iters, seed, sm_count, nv, handle):
    matmul = torch.backends.cuda.matmul
    if not hasattr(matmul, "allow_fp16_accumulation"):
        return None
    gen = torch.Generator(device=device).manual_seed(seed)
    a = torch.randn(n, n, device=device, dtype=torch.float16, generator=gen)
    b = torch.randn(n, n, device=device, dtype=torch.float16, generator=gen)
    out = torch.empty(n, n, device=device, dtype=torch.float16)
    ref = a[:16, :].cpu().double() @ b[:, :16].cpu().double()
    op = lambda: torch.mm(a, b, out=out)

    old_fp16_acc = matmul.allow_fp16_accumulation
    blocks = []
    try:
        for enabled in (False, True, True, False):
            matmul.allow_fp16_accumulation = enabled
            burst = _run_steady_burst(
                torch, op, n, warmup, min_iters, min_seconds, max_iters, nv, handle, device_index
            )
            finite, rel_l2 = _sample_accuracy(torch, out, ref)
            fsm = flop_per_clk_sm(burst["tflops"], burst["sm_clock_mhz"], sm_count)
            blocks.append({
                "allow_fp16_accumulation": enabled,
                "flop_clk_sm": fsm,
                "sample_finite": finite,
                "sample_relative_l2_error": rel_l2,
                **burst,
            })
    finally:
        matmul.allow_fp16_accumulation = old_fp16_acc

    def pair_ratio(off_blk, on_blk):
        raw_x = on_blk["tflops"] / off_blk["tflops"] if off_blk["tflops"] else None
        norm_x = (
            on_blk["flop_clk_sm"] / off_blk["flop_clk_sm"]
            if (on_blk["flop_clk_sm"] and off_blk["flop_clk_sm"])
            else None
        )
        return {"raw_tflops_ratio": raw_x, "clock_normalized_ratio": norm_x}

    return {
        "order": ["off", "on", "on", "off"],
        "allow_fp16_reduced_precision_reduction": getattr(
            matmul, "allow_fp16_reduced_precision_reduction", None
        ),
        "blocks": blocks,
        "pair_off_then_on": pair_ratio(blocks[0], blocks[1]),
        "pair_on_then_off": pair_ratio(blocks[3], blocks[2]),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--device", type=int, default=0, help="CUDA device index (default: 0)")
    ap.add_argument("--n", type=int, default=8192, help="square matrix size N (default: 8192)")
    ap.add_argument("--warmup", type=int, default=8, help="warmup GEMMs per case (default: 8)")
    ap.add_argument("--min-iters", type=int, default=30, help="minimum timed GEMMs per case (default: 30)")
    ap.add_argument("--min-seconds", type=float, default=2.0, help="target steady-state window per case in seconds (default: 2.0)")
    ap.add_argument("--max-iters", type=int, default=400, help="upper cap on timed GEMMs per case (default: 400)")
    ap.add_argument("--seed", type=int, default=1234, help="PRNG seed for matrices (default: 1234)")
    ap.add_argument("--no-paired-fp16", action="store_true", help="skip the order-balanced FP16 accumulation A/B block")
    ap.add_argument("--json", type=Path, default=None, help="write structured JSON report to path")
    args = ap.parse_args()

    if min(args.n, args.warmup, args.min_iters, args.max_iters) < 1 or args.min_seconds <= 0:
        ap.error("--n, --warmup, --min-iters, --max-iters, and --min-seconds must be positive")

    import torch

    if not torch.cuda.is_available() or torch.version.cuda is None:
        ap.error("a CUDA-enabled PyTorch build and an NVIDIA GPU are required")
    if args.device < 0 or args.device >= torch.cuda.device_count():
        ap.error(f"--device {args.device} out of range (found {torch.cuda.device_count()} CUDA device(s))")

    torch.cuda.set_device(args.device)
    device = f"cuda:{args.device}"
    props = torch.cuda.get_device_properties(args.device)
    cc = (props.major, props.minor)
    ak = arch_key(*cc)
    smi_meta = query_nvidia_smi_meta(args.device)
    nv, handle = nvml_handle(args.device)

    info = {
        "os": platform.system(),
        "os_release": platform.release(),
        "torch": str(torch.__version__),
        "cuda": torch.version.cuda,
        "gpu": props.name,
        "compute_capability": [props.major, props.minor],
        "sm_count": props.multi_processor_count,
        "arch_key": ak,
        "total_mem_mib": round(props.total_memory / (1024 ** 2)),
        **smi_meta,
    }
    print("=" * 84)
    print(f"GPU            : {info['gpu']} (cc {cc[0]}.{cc[1]}, {info['sm_count']} SMs, arch={ak})")
    print(f"Driver / VBIOS : {info.get('driver', 'n/a')} / {info.get('vbios', 'n/a')} | PCI {info.get('pci_device_id', 'n/a')}")
    print(f"PyTorch / OS   : {info['torch']} (CUDA {info['cuda']}) on {info['os']} {info['os_release']}")
    print(f"Config         : N={args.n}, warmup={args.warmup}, min_iters={args.min_iters}, min_s={args.min_seconds}")
    print("=" * 84)

    cases = [
        "fp16_fp16acc",
        "fp16_fp32acc",
        "bf16_fp32acc",
        "tf32_fp32acc",
        "fp8_fp32acc",
        "int8",
        "fp32_control",
    ]
    full_rates = FULL_RATE_FLOP_CLK_SM.get(ak, {})
    rows = []
    for case in cases:
        res = bench_case(
            torch,
            device,
            args.device,
            args.n,
            case,
            args.warmup,
            args.min_iters,
            args.min_seconds,
            args.max_iters,
            args.seed,
            nv,
            handle,
        )
        if res is None:
            continue
        fsm = None if case == "fp32_control" else flop_per_clk_sm(res["tflops"], res["sm_clock_mhz"], info["sm_count"])
        full = full_rates.get(case) if case != "fp32_control" else None
        if full == 0:
            full = None
        ratio = (fsm / full) if (fsm and full) else None
        verdict = classify_case(case, ratio)
        res["flop_clk_sm"] = fsm
        res["full_rate_flop_clk_sm"] = full
        res["ratio_of_full"] = ratio
        res["verdict"] = verdict
        rows.append(res)

    hdr = (
        f"{'case':14s}{'iters':>6}{'T(FL)OP/s':>10}{'clk MHz':>9}{'W':>7}"
        f"{'FLOP/c/SM':>11}{'full':>6}{'ratio':>7}{'relL2':>10}  verdict"
    )
    print(hdr)
    print("-" * len(hdr))
    for r in rows:
        clk_s = f"{r['sm_clock_mhz']:.0f}" if r["sm_clock_mhz"] else "n/a"
        pw_s = f"{r['power_w']:.1f}" if r["power_w"] else "n/a"
        fsm_s = f"{r['flop_clk_sm']:.1f}" if r["flop_clk_sm"] else "-"
        full_s = f"{r['full_rate_flop_clk_sm']}" if r["full_rate_flop_clk_sm"] else "-"
        rat_s = f"{r['ratio_of_full']:.2f}" if r["ratio_of_full"] else "-"
        err_s = f"{r['sample_relative_l2_error']:.2e}" if r["sample_relative_l2_error"] is not None else "-"
        print(
            f"{r['case']:14s}{r['iters']:6d}{r['tflops']:10.2f}{clk_s:>9}{pw_s:>7}"
            f"{fsm_s:>11}{full_s:>6}{rat_s:>7}{err_s:>10}  {r['verdict']}"
        )

    paired = None
    if not args.no_paired_fp16:
        paired = run_paired_fp16_ab(
            torch,
            device,
            args.device,
            args.n,
            args.warmup,
            args.min_iters,
            args.min_seconds,
            args.max_iters,
            args.seed,
            info["sm_count"],
            nv,
            handle,
        )
        if paired is not None:
            print("\n--- Order-balanced FP16 accumulation A/B (off -> on -> on -> off) ---")
            for idx, blk in enumerate(paired["blocks"]):
                tag = "ON " if blk["allow_fp16_accumulation"] else "OFF"
                clk_s = f"{blk['sm_clock_mhz']:.0f} MHz" if blk["sm_clock_mhz"] else "n/a"
                pw_s = f"{blk['power_w']:.1f} W" if blk["power_w"] else "n/a"
                fsm_s = f"{blk['flop_clk_sm']:.1f}" if blk["flop_clk_sm"] else "n/a"
                err_s = f"{blk['sample_relative_l2_error']:.6f}" if blk["sample_relative_l2_error"] is not None else "n/a"
                print(
                    f"  block {idx + 1} [fp16_acc={tag}]: {blk['tflops']:6.2f} TFLOP/s | "
                    f"{clk_s:>9} | {pw_s:>7} | {fsm_s:>6} FLOP/clk/SM | relL2={err_s}"
                )
            p1 = paired["pair_off_then_on"]
            p2 = paired["pair_on_then_off"]
            print(
                f"  off->on ratio: raw={p1['raw_tflops_ratio']:.2f}x, "
                f"clock-norm={p1['clock_normalized_ratio']:.2f}x | "
                f"on->off ratio: raw={p2['raw_tflops_ratio']:.2f}x, "
                f"clock-norm={p2['clock_normalized_ratio']:.2f}x"
            )

    report = {
        "schema": "nvuu-clamp-eval/1",
        "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "info": info,
        "config": {
            "device": args.device,
            "n": args.n,
            "warmup": args.warmup,
            "min_iters": args.min_iters,
            "min_seconds": args.min_seconds,
            "max_iters": args.max_iters,
            "seed": args.seed,
        },
        "results": rows,
        "paired_fp16_accumulation": paired,
    }
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8")
        print(f"\nwrote {args.json}")


if __name__ == "__main__":
    main()
