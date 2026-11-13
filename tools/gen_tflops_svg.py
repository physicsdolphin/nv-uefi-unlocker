#!/usr/bin/env python3
"""Render a static SVG snapshot of docs/tools/tflops-ceiling.html.

The HTML is the single source of truth (its embedded data array); this script
only re-draws one precision column as an image the READMEs can embed.

Usage: python -I -X utf8 tools/gen_tflops_svg.py [--precision fp16] [--top 18] [--lang zh|en]
"""

import argparse
import json
import re
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "docs" / "tools" / "tflops-ceiling.html"

PRECISIONS = {
    "fp32": ("f", "FP32", ("CUDA cores · shipped at full speed, unlock changes nothing",
                           "CUDA 核 · 出厂即全速，解锁不改变")),
    "tf32": ("t", "TF32", ("Tensor cores · 30/40-series both ship at 1× FP32, unlock is 2×",
                           "张量核 · 30/40 系都出厂砍到 1× FP32，解锁才回 2×")),
    "fp16": ("h", "FP16", ("Tensor cores · FP32 accumulate · halved from factory",
                           "张量核 · FP32 累加 · 出厂被砍半")),
    "bf16": ("b", "BF16", ("Tensor cores · FP32 accumulate · halved from factory",
                           "张量核 · FP32 累加 · 出厂被砍半")),
    "fp8": ("e", "FP8", ("Tensor cores · FP32 accumulate · halved from factory · 40-series only",
                         "张量核 · FP32 累加 · 出厂被砍半 · 仅 40 系")),
}

TEXT = {
    "en": {
        "title": "Theoretical post-unlock ceiling · {label}",
        "sub": "{blurb} · dense TFLOPS · no sparsity",
        "legend": "grey = as shipped (clamped) · colour = theoretical post-unlock ceiling · peak {peak:.1f} TFLOPS",
        "hidden": "……{n} more not drawn; the interactive version has all of them: docs/tools/tflops-ceiling.html",
        "f1": "value = SM × FMA/SM/clk × 2 × boost clock; unlock restores the whitepaper rate (×2 on clamped paths)",
        "f2": "real results land lower: power limits, memory bandwidth and library routing each take a cut",
        "f3": "consumer 30/40-series TF32 is factory-clamped too (measured 2026-11-06) · desktop consumer only; laptops, 9/10/16 series, pro, mobile-workstation, datacenter and RTX 50 rows are interactive-only",
        "aria": "Theoretical post-unlock ceiling chart · {label}",
    },
    "zh": {
        "title": "理论解锁上限 · {label}",
        "sub": "{blurb} · dense TFLOPS · 不含稀疏",
        "legend": "灰 = 出厂（钳制） · 彩 = 解锁后理论上限 · 峰值 {peak:.1f} TFLOPS",
        "hidden": "……另有 {n} 款未画出，完整可交互版见 docs/tools/tflops-ceiling.html",
        "f1": "值为 SM × 每 SM 每时钟 FMA × 2 × 加速频率；解锁 = 恢复白皮书全速，钳制路径 ×2",
        "f2": "实际到手低于此值：功耗墙、显存带宽、库路由各自打折 · 消费 30/40 系 TF32 同被出厂钳制（2026-11-06 实测）",
        "f3": "消费 30/40 系 TF32 同样被出厂钳制（2026-11-06 实测）· 仅列桌面消费卡；笔记本、9/10/16 系、专业卡、移动工作站、数据中心与 50 系对照行见交互版",
        "aria": "理论解锁上限天梯图 · {label}",
    },
}

GEN_COLOR = {"9": "#78350f", "10": "#92400e", "16": "#d97706", "20": "#4d7c0f", "30": "#0e7490", "40": "#c2410c"}
GEN_COLOR_DARK = {"9": "#b45309", "10": "#92400e", "16": "#d97706", "20": "#a3e635", "30": "#22d3ee", "40": "#fb923c"}


def load_rows() -> list[dict]:
    html = SRC.read_text(encoding="utf-8")
    m = re.search(r"var D=(\[.*?\]);\n", html, re.S)
    if not m:
        raise SystemExit("data array not found in " + str(SRC))
    return json.loads(m.group(1))


def bar(x0: float, y: float, w: float, h: float, cls: str) -> str:
    return f'<rect class="{cls}" x="{x0:.1f}" y="{y:.1f}" width="{max(w, 0):.1f}" height="{h}" rx="2"/>'


def text_width(text: str) -> float:
    """Rough advance estimate at font-size 11; only used to clip chip notes."""
    return sum(11.5 if ord(ch) > 0x2E7F else 6.2 for ch in text)


def render(precision: str, top: int, lang: str) -> str:
    key, label, (blurb_en, blurb_zh) = PRECISIONS[precision]
    T = TEXT[lang]
    blurb = blurb_en if lang == "en" else blurb_zh
    rows = [r for r in load_rows()
            if r.get(key) and not r.get("m") and not r.get("k") and not r.get("ref")]
    rows.sort(key=lambda r: -r[key][1])
    peak = rows[0][key][1]

    shown = rows[:top]
    hidden = len(rows) - len(shown)

    W, RH = 720, 22
    y0 = 104
    H = y0 + len(shown) * RH + (26 if hidden else 0) + 94
    tx0, tw = 252, 330

    out: list[str] = []
    out.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
        f'viewBox="0 0 {W} {H}" role="img" aria-label="{escape(T["aria"].format(label=label))}">'
    )
    out.append("<style>")
    out.append(
        ".bg{fill:#fff}.fg{fill:#111}.mut{fill:#666}.brd{fill:none;stroke:#ddd}"
        ".trk{fill:#eceef0}"
        f".g20{{fill:{GEN_COLOR['20']}}}.g30{{fill:{GEN_COLOR['30']}}}.g40{{fill:{GEN_COLOR['40']}}}"
        f".g9{{fill:{GEN_COLOR['9']}}}.g10{{fill:{GEN_COLOR['10']}}}.g16{{fill:{GEN_COLOR['16']}}}"
        ".lock{fill:#b9bec4}"
        "text{font-family:system-ui,-apple-system,'Segoe UI',sans-serif}"
        "@media (prefers-color-scheme:dark){.bg{fill:#111417}.fg{fill:#e6e8ea}.mut{fill:#9aa3ad}"
        ".brd{stroke:#333a42}.trk{fill:#232a31}.lock{fill:#5b636c}"
        f".g20{{fill:{GEN_COLOR_DARK['20']}}}.g30{{fill:{GEN_COLOR_DARK['30']}}}.g40{{fill:{GEN_COLOR_DARK['40']}}}"
        f".g9{{fill:{GEN_COLOR_DARK['9']}}}.g10{{fill:{GEN_COLOR_DARK['10']}}}.g16{{fill:{GEN_COLOR_DARK['16']}}}}}"
    )
    out.append("</style>")
    out.append(f'<rect class="bg" width="{W}" height="{H}" rx="10"/>')

    out.append(f'<text class="fg" x="20" y="36" font-size="16" font-weight="600">'
               f'{escape(T["title"].format(label=label))}</text>')
    out.append(f'<text class="mut" x="20" y="57" font-size="12">{escape(T["sub"].format(blurb=blurb))}</text>')
    out.append(f'<text class="mut" x="20" y="78" font-size="12">{escape(T["legend"].format(peak=peak))}</text>')
    out.append(f'<line class="brd" x1="20" y1="{y0 - 12}" x2="{W - 20}" y2="{y0 - 12}"/>')

    for i, r in enumerate(shown):
        y = y0 + i * RH
        lo, hi = r[key]
        gen = r["g"]
        out.append(f'<text class="mut" x="34" y="{y + 12}" font-size="11" text-anchor="end">{i + 1}</text>')
        name_s = r["n"].replace("GeForce RTX ", "RTX ")
        name = escape(name_s)
        suffix = " †" if precision == "tf32" and (r.get("ref") or gen == "50") else ""
        x_name = 44 + 7.2 * len(name_s) + 6
        label = f"{r['c']} · {r['s']} SM{suffix}"
        if text_width(label) <= tx0 - 8 - x_name:
            out.append(f'<text class="fg" x="44" y="{y + 12}" font-size="12">{name}</text>')
            out.append(f'<text class="mut" x="44" y="{y + 12}" font-size="11">'
                       f'<tspan x="{x_name}">{escape(label)}</tspan></text>')
        else:
            # Long name leaves no column for the chip note (variant rows); keep
            # the SM count inline so it is never hidden under the bar.
            out.append(f'<text class="fg" x="44" y="{y + 12}" font-size="12">{name}'
                       f'<tspan class="mut" font-size="11"> · {r["s"]} SM{suffix}</tspan></text>')
        out.append(f'<rect class="trk" x="{tx0}" y="{y + 5}" width="{tw}" height="9" rx="2"/>')
        out.append(bar(tx0, y + 5, tw * hi / peak, 9, f"g{gen}"))
        out.append(bar(tx0, y + 5, tw * lo / peak, 9, "lock"))
        # A bar that starts and ends at the same place needs its coloured tip back.
        if lo == hi:
            out.append(bar(tx0, y + 5, tw * hi / peak, 9, f"g{gen}"))
        val = f"{lo:.1f} → {hi:.1f}" if lo != hi else f"{hi:.1f}"
        out.append(f'<text class="fg" x="{W - 20}" y="{y + 12}" font-size="11" text-anchor="end">{val}</text>')

    if hidden:
        out.append(
            f'<text class="mut" x="20" y="{y0 + len(shown) * RH + 14}" font-size="11">'
            f'{escape(T["hidden"].format(n=hidden))}</text>'
        )

    foot = H - 46
    out.append(f'<line class="brd" x1="20" y1="{foot - 14}" x2="{W - 20}" y2="{foot - 14}"/>')
    out.append(f'<text class="mut" x="20" y="{foot}" font-size="11">{escape(T["f1"])}</text>')
    out.append(f'<text class="mut" x="20" y="{foot + 16}" font-size="11">{escape(T["f2"])}</text>')
    out.append(f'<text class="mut" x="20" y="{foot + 32}" font-size="11">{escape(T["f3"])}</text>')
    out.append("</svg>")
    return "\n".join(out)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--precision", default="fp16", choices=sorted(PRECISIONS))
    ap.add_argument("--top", type=int, default=18)
    ap.add_argument("--lang", default="zh", choices=sorted(TEXT))
    args = ap.parse_args()
    name = "tflops-ceiling.en.svg" if args.lang == "en" else "tflops-ceiling.svg"
    out = ROOT / "docs" / "tools" / name
    out.write_text(render(args.precision, args.top, args.lang), encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)} ({args.precision}, top {args.top}, {args.lang})")


if __name__ == "__main__":
    main()
