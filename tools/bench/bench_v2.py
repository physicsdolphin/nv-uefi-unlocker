# bench_v2.py - community-standard 4-precision GEMM measurement (FP16/BF16/TF32 + FP32 control)
# Vendored 2026-11-09 from @cyhu9527 (issue #198 run-11/12 bundle, MIT-assumed-by-contribution to the project docs; script by cyhu9527).
# Provenance: https://github.com/cyhu9527/y9000p-nv-unlocker-evidence (release run11)
# Reported use: Y9000P 4090 Laptop (2757) locked-vs-unlocked A/B, 400-iter medians.
# NOTE: the header comment below says "20 iters"; the shipped default is BENCH_ITERS=400
#       (the value the reference A/B used). Kept byte-for-byte apart from this banner.

# GPU tensor-limiter bench, community-comparable: FP16/BF16/TF32 + FP32 control.
# 4096^3 GEMM, 20 iters median per dtype, clock+power sampled during each segment.
# Usage: set BENCH_TAG=xxx & set BENCH_OUT=path.txt & python bench_v2.py
# Knobs via env: BENCH_ITERS (20), BENCH_N (4096). torch-only, no numpy.
import os, sys, time, threading, subprocess, statistics

TAG = os.environ.get('BENCH_TAG', 'unlabeled')
OUT = os.environ.get('BENCH_OUT', '')
ITERS = int(os.environ.get('BENCH_ITERS', '400'))
N = int(os.environ.get('BENCH_N', '4096'))
WARMUP = 3

import torch

lines = []
def emit(s):
    print(s, flush=True)
    lines.append(s)

props = torch.cuda.get_device_properties(0)
emit(f'TAG={TAG}')
emit(f'at={time.strftime("%Y-%m-%d %H:%M:%S")}')
emit(f'torch={torch.__version__} cuda={torch.version.cuda}')
emit(f'dev={props.name} cc={props.major}.{props.minor} vram={props.total_memory >> 30}GB')
emit(f'N={N} iters={ITERS} warmup={WARMUP}')

stop = False
samples = {}
def nvsmi():
    try:
        r = subprocess.run(['nvidia-smi', '--query-gpu=clocks.sm,power.draw',
                            '--format=csv,noheader,nounits'],
                           capture_output=True, text=True, timeout=10)
        c, p = r.stdout.strip().split(',')
        return int(c), float(p)
    except Exception:
        return None

def sampler():
    while not stop:
        s = nvsmi()
        if s:
            samples['clock'].append(s[0]); samples['power'].append(s[1])
        time.sleep(0.2)

def bench(dtype_name):
    global stop
    samples['clock'] = []
    samples['power'] = []
    dt = {'fp16': torch.float16, 'bf16': torch.bfloat16,
          'tf32': torch.float32, 'fp32': torch.float32}[dtype_name]
    torch.backends.cuda.matmul.allow_tf32 = (dtype_name == 'tf32')
    torch.backends.cudnn.allow_tf32 = (dtype_name == 'tf32')
    torch.manual_seed(1234)
    a = torch.randn(N, N, device='cuda', dtype=dt)
    b = torch.randn(N, N, device='cuda', dtype=dt)
    stop = False
    th = threading.Thread(target=sampler, daemon=True)
    th.start()
    times = []
    c = None
    for i in range(ITERS + WARMUP):
        torch.cuda.synchronize()
        t0 = time.perf_counter()
        c = a @ b
        torch.cuda.synchronize()
        t1 = time.perf_counter()
        if i >= WARMUP:
            times.append(t1 - t0)
    stop = True
    th.join(timeout=3)
    post = nvsmi()
    if post:
        samples['clock'].append(post[0]); samples['power'].append(post[1])
    finite = bool(torch.isfinite(c).all().item())
    af = torch.randn(64, 512, device='cuda', dtype=dt)
    bf = torch.randn(512, 64, device='cuda', dtype=dt)
    ref = af.to(torch.float64) @ bf.to(torch.float64)
    got = (af @ bf).to(torch.float64)
    tol = 0.35 if dt in (torch.float16, torch.bfloat16) else 0.2
    rel = ((got - ref).abs() / (ref.abs() + 1.0)).max().item()
    ok = finite and rel < tol
    del a, b, c, af, bf, ref, got
    torch.cuda.empty_cache()
    tmed = statistics.median(times)
    tf = 2.0 * N ** 3 / tmed / 1e12
    clk = samples.get('clock', [0]); pw = samples.get('power', [0.0])
    emit(f'{dtype_name}: median={tmed*1000:.2f}ms FP={tf:.2f} TFLOPS '
         f'clock_med={int(statistics.median(clk))}/{max(clk)}MHz '
         f'power_med={statistics.median(pw):.0f}/{max(pw):.0f}W '
         f'relerr={rel:.4f} ok={ok}')

for d in ('fp16', 'bf16', 'tf32', 'fp32'):
    bench(d)
torch.backends.cuda.matmul.allow_tf32 = False
emit(f'BENCH_DONE {TAG}')

if OUT:
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines) + '\n')
