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

## Milestones

- **G0** position-independent fuc5 decoder (opcode authority:
  `reference/envytools/envydis/falcon.c`, one-shot transcription to Rust;
  envydis stays as the cross-check oracle) + scanner completion. Accept:
  6/6 on the TU102 sites, all known GA102/AD102 code entries, zero
  divergence vs envydis.
- **G1** stack-geometry derivation (static interprocedural frame analysis;
  emulator is the fallback — note faucon has NO interpreter, public
  tooling has none either). Accept: 0xFF38/40/44/4C, stride 0x18,
  0x600/0xFFF0 derived from the plaintext alone.
- **G2** minimal-chain synthesis + golden-table reproduction (byte-exact
  or per-word justified).
- **G3** full-parity tail frames + first fresh-die use (TU116 cross-check
  against the hand-anchored table).

G0–G2 are fully offline, zero GPU contact (constitution §1.8: sim first).

## In this directory (bootstrap stage)

- `gadget_scan.py` — the 150-line prototype (moved from scratch). Harvests
  an instruction-length table from the envydis lst, decodes both parities
  from every byte offset, classifies ROP-terminated slices. Prototype
  result: auto-recovers 4/6 TU102 + 2/5 GA102 known sites without being
  told them; misses = mid-entry opcode starts absent from the canonical
  stream (exactly the gap G0's real decoder closes).

```text
GADGET_LST=<sec.lst> python -I gadget_scan.py <sec_plain.bin> 0x100 [known VAs...]
```

Next step (G0): port the decoder to Rust under this directory as a crate,
harvest the fuc5 opcode/encoding tables from `falcon.c`, wire the
length-table bootstrap as the test oracle.
