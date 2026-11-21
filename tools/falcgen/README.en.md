# falcgen — first-principles V67 payload generator · [中文](README.md)

A position-independent fuc5 decoder + narrow interpreter + gadget scanner +
**payload synthesizer**: it consumes only a decrypted booter plaintext,
derives the stack geometry automatically, and synthesizes a byte-verifiable
V67 payload — **the write chain is the payload's only policy input**
(`writes = [(addr, value), ...]`). Feasibility study:
[docs/first-principles-payload-generator.md](../../docs/first-principles-payload-generator.md).

A crate detached from the repo-root workspace (same lane as
[tools/ssdiag](../ssdiag/): its own version 0.1.0 / Cargo.lock / target dir,
designed for a standalone release). **Zero dependencies, fully offline**:
`synth` needs no NVIDIA assets and no input files; payloads/goldens/plaintexts
never enter git.

## Quick start

```bash
# inside tools/falcgen/ (or add --manifest-path tools/falcgen/Cargo.toml)
cargo build --release        # artifact: tools/falcgen/target/release/falcgen.exe
cargo test                   # unit + pinned-SHA golden self-tests (always run)

# Per-family SS unlock-chain presets (outputs pinned byte-for-byte by
# tests/goldens.rs against the pinned SHAs)
falcgen synth --family tu11x --gate ss                 # = c79208ac… (pinned)
# AD102 BAR1 64G (= cf3566d0…, the eight-card artifact, PROVEN; other 2^n
# sizes emit an UNPROVEN manifest flag)
falcgen synth --family ad102 --gate bar1 --bar1-size 64 --out payload.bin
# Raw write chain (passes the frame-budget check; over-budget chains are
# refused with the per-die limit table)
falcgen synth --family tu102 --writes 0x409650=0xffffffff,0x1fa828=0
```

`--out` also writes a manifest sidecar `<out>.manifest.txt`: write chain,
marker, expected readbacks, preconditions (FEAT PLM write-enable nibble
[7:4]==8 check, Turing relock-tail note), UNPROVEN flags. Placing a payload
on a stick is a user-gated decision; executing it on hardware is another.

## Subcommands

| Command | Purpose |
|---|---|
| `dis` | Position-independent fuc5 disassembly (`--crypt` enables the CRYPT feature bit; `--all-offsets` decodes every byte offset for mid-entry gadget hunting) |
| `scan` | Gadget classification (forward-walk, unknown-tolerant; 17/17 known entry sites hit) |
| `lst-compare` | envydis oracle diff (offset/oplen/mnemonic triples, zero-divergence acceptance) |
| `run` | Narrow fuc5 interpreter (plaintext-fed, MMIO-stubbed, DMA-as-copy; `--break`/`--trace`/`--no-stop-after-dma`) |
| `synth` | Payload synthesis: raw write chain or gate preset + frame-budget check + golden diff + manifest |

## Milestones (all landed)

- **G0** Position-independent decoder (Rust transcription of envydis/falcon.c,
  18 tables, + core-dis semantics): 54,670 instructions across five decrypted
  booter segments, zero mismatches; 360 sampled non-canonical offsets agree
  per-offset (env-gated oracle). Erratum: the "unknown bytes" were
  **crypt-gated instructions, not a table gap** — with crypt on, all five
  segments decode with zero unknowns; the SEC entry opens with `csigclr` +
  a csecret/cxor key-zeroing sequence, so the scanner and interpreter run
  `FUC5_CRYPT` throughout.
- **G1** Stack-geometry derivation (interpreter trace): live frames
  0xFF38/40/44/4C, stride 0x18, copy base 0x600, main SP 0xFFF0 — all
  trace-proven.
- **G2** Golden tables reproduced byte-for-byte: TU102 `b0ec9029…` /
  GA102 `2a0aa1f8…` / AD102 `44c36b2d…`.
- **G3** TU116, the first fully first-principles die (differs from the
  hand-anchored `c79208ac…` table by exactly 4 operand words) + the first
  novel product: a direct SS0/SS1 chain (`1db35f51…`).
- **Golden self-tests need no golden files**: synth is a pure function, so
  `tests/goldens.rs` pins the preset outputs to the pinned SHAs — the digest
  is the proof.

## What this adds (vs. hand-editing constants)

1. **Zero-touch cross-die porting** — geometry (gadget sites, live-frame
   slots, stride, stack layout) derived from the decrypted booter segment
   alone; the marginal cost of a new die = one debug key + one command.
   TU116 (GTX 16 series) is the first die produced with no upstream table.
2. **Automated validation for beyond-grammar structures** — many-write
   chains, tail re-layout, alternate teardowns: golden reproduction plus the
   `sim/` behavioral harness (`sim::run_payload_chain` through the PLM
   privilege model), not try-and-read-back.
3. **Extending the reachable surface itself** — new primitives (widths,
   DMA-carried writes, read-modify-write) require new gadget derivation;
   G0/G1 are the prerequisites.

Hard boundary: a die without a broken debug key has no generator input
(TU = key family #6, GA = #37 are broken; AD/GA100 plaintexts on file). The
per-die write budget (tu102/tu11x=2, ga102=1, ad102=4) is a mechanical
boundary — larger chains need pivot re-layout (not implemented); see the
[gate inventory and smashing-boundary ledger](../../docs/gate-inventory.md).

## Discipline

- Constitution §1.8: fully offline, zero GPU contact; hardware execution is a
  separately gated decision (milestone gate + explicit confirmation).
- Turing handoff: PLM-open handoff is refused by Linux 580.x GSP-RM
  (issue #246); the relock tail `0xffffffef` with SS intact is the only
  proven shape (the manifest notes this per family).
- The payload's DMEM constants are backed on prod by "the upstream tables are
  field-proven"; when a pinned version changes, re-review exploitability as a
  first-class property.

## Docs and layout

- Manual and design rationale: [docs/falcgen-manual.md](../../docs/falcgen-manual.md)
- Gate inventory and smashing boundary (horizontal-expansion ledger):
  [docs/gate-inventory.md](../../docs/gate-inventory.md)
- `src/`: isa/decode/scan/interp/synth + CLI; `tests/oracle.rs`: envydis
  ground-truth tests (env-gated: `FALCGEN_SEC_DIR` points at the RE
  workspace plaintexts, `FALCGEN_ENVYDIS` is a WSL envydis prefix — wsl.exe
  stdin piping from Windows is not binary-safe, so the test runs a script
  inside WSL). Silent skip when unset.
- `gadget_scan.py`: historical Python prototype (superseded by
  `src/scan.rs`), kept as feasibility evidence.

License: GPL-2.0-only (repo-root LICENSE).
