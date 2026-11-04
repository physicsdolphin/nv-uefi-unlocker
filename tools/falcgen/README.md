# falcgen — first-principles V67 payload generator (research branch)

Branch `falcon-booter-payload-gen`. Feasibility study:
[docs/first-principles-payload-generator.md](../../docs/first-principles-payload-generator.md)
(frame algorithm fully recovered; G0–G3 plan; golden-standard protocol).

## Scope statement — what this project does and does not unlock

**Already possible WITHOUT this project** (read-back is the safety net, so
this is not a gate): on any die with audited geometry (GA102 / TU102 /
AD102, and TU116 via the hand audit in `assets/src/payload.rs`), a custom
write chain is a *data-level edit* — the frame grammar treats the write
list as the only policy input (`writes = [(addr, value), ...]`, one
0x18-byte frame per write, tail blocks relocatable). The frame dance
(canary passing, epilogue cascade) is untouched, so a wrong target simply
fails read-back. Anything the current write primitive can reach (32-bit
PRI writes from the SEC2 heavy-secure context) is addressable today.

**What this project adds:**

1. **Zero-touch cross-die porting** — geometry derivation (gadget sites,
   live-frame slots, stride, stack layout) from the decrypted booter
   segment alone, instead of one manual RE round per die.
2. **Automated validation for beyond-grammar structures** — many-write
   chains, tail re-layout, different teardowns: golden-table reproduction
   (GA102 `2a0aa1f8…` / TU102 `b0ec9029…` / AD102 `44c36b2d…`) plus the
   `sim/` harness instead of try-and-read-back.
3. **Extending the reachable surface itself** — new primitives (widths,
   DMA-carried writes, read-modify-write) require new gadget derivation:
   the position-independent fuc5 decoder (G0) and stack-geometry
   derivation (G1) are the prerequisites.

Hard boundary: a die without a broken debug key has no generator input
(TU = key family #6, GA = #37 are broken; AD/GA100 plaintexts on file).

## G0 — LANDED (2026-11-05)

- **Position-independent fuc5 decoder** (`src/isa.rs` + `src/decode.rs`):
  full Rust transcription of envydis/falcon.c (18 tables, ~380 entries) +
  core-dis.c engine semantics. Validation: linear decode reproduces the
  envydis lst **exactly on all five decrypted booter segments**
  (TU102/TU116/GA102/AD102/GA100, 54,670 instructions, zero mismatches)
  and 360 sampled non-canonical offsets agree per-offset (env-gated
  oracle tests, silent skip without the RE workspace).
- **Scanner** (`src/scan.rs`): forward-walk classification on decoded
  operand kinds, unknown-tolerant. Acceptance **17/17** known code-entry
  sites — TU102 6/6 (incl. the mid-entry `0x10b9`/`0xc95` the python
  prototype missed), GA102 5/5, AD102 6/6.
- `falcgen dis|scan|lst-compare` CLI; `cargo test -p falcgen` green;
  `cargo xt` green.

**CORRECTED G0 finding (2026-11-05, crypt discovery)**: the "unknown
bytes" were **crypt-gated instructions, not a table gap** — envydis's
fuc5 table is complete once the CRYPT feature is enabled. With crypt on,
all five decrypted booter segments decode with **zero unknown
instructions** (the lst files were generated without it; the ??? clusters
are `cxset`/`cocmd`/`csigclr`-family crypt ops — the SEC entry even opens
with `csigclr` + a csecret/cxor key-zeroing sequence). The scanner and
interpreter therefore run `FUC5_CRYPT`. cmp50hx's "archive bytes are not
runtime gadget bytes" remains true independently (different keystream).

## Milestones

- **G1** stack-geometry derivation — dual track (user decision):
  Track A static interprocedural frame analysis over the decoded stream;
  Track B a narrow fuc5 interpreter (plaintext-fed, MMIO-stubbed,
  DMA-as-copy) whose reuse value extends into `sim/` semantic
  validation. Accept either way: derived geometry == golden values
  (TU102: 0xFF38/40/44/4C, stride 0x18, copy base 0x600, main SP 0xFFF0).
- **G2** minimal-chain synthesis + golden-table reproduction (byte-exact
  or per-word justified; fill-word derivation mechanical via the
  decoder).
- **G3** full-parity tail frames + first fresh-die use (TU116 cross-check
  against the hand-anchored table).

G0–G2 are fully offline, zero GPU contact (constitution §1.8: sim first).

## In this directory

- `src/` — the decoder, scanner and CLI (`cargo run -p falcgen -- dis
  <file> --base 0x100 [--all-offsets] | scan <file> --base <va>
  --check <va>,... | lst-compare <plain> <lst> --base 0x100`).
- `tests/oracle.rs` — envydis ground-truth tests (env-gated):
  `FALCGEN_SEC_DIR=D:/git-repo/nv-uefi-re` and
  `FALCGEN_ENVYDIS=/mnt/d/.../envydis` (a WSL path; the test runs envydis
  inside WSL because wsl.exe stdin piping from Windows is not
  binary-safe). Silent skip when unset.
- `gadget_scan.py` — HISTORICAL python prototype (superseded by
  `src/scan.rs`), kept as the feasibility evidence.

Next step (G1): dual-track stack-geometry derivation per the milestone
note above; the checkpoint after G0 re-plans the details.
