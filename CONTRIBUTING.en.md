# Contributing

> 中文版：[CONTRIBUTING.md](CONTRIBUTING.md)

Thanks for your interest in this project — a GPU firmware security research tool (GPL-2.0-only). Before starting, please read the [README](README.en.md) and the [FAQ](docs/faq.en.md).

## Where should I start?

| What you want to do | Where to go |
|---|---|
| Share hardware test results (compatibility / readback / performance) | [Hardware report template](.github/ISSUE_TEMPLATE/02-hardware-report.yml) |
| Report a failure in stick prep, EFI execution, OS handoff, or the diagnostic tool | [Bug report template](.github/ISSUE_TEMPLATE/01-bug-report.yml) |
| Suggest a feature or documentation improvement | [Request template](.github/ISSUE_TEMPLATE/03-request.yml) |
| Ask usage questions / open discussion | [Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions) |
| Change code or docs | The rest of this file |

**Hardware test data is the most valuable contribution**: the per-device compatibility matrix is built entirely from community reports. You can normalize `loader.log` / ssdiag output into JSON with `tools/collect_compat.py` and attach it to your report; maintainers update the matrix and labels from that evidence.

## Before submitting code

### Building

Same steps as [Build from source](README.en.md#build-from-source-optional) in the README:

```
rustup target add x86_64-unknown-uefi
cargo efi          # loader release UEFI build -> target/x86_64-unknown-uefi/release/loader.efi
cargo efi-debug    # loader debug build
cargo xt           # all host-side tests (assets + sim; loader is not host-testable)
python check_efi.py <loader.efi>   # banner / build-class verification
```

The aliases come from `.cargo/config.toml`. Repository layout: `loader/` (UEFI app), `assets/` (asset pipeline), `sim/` (behavioral simulator), `tools/` (host-side diagnostics).

### Hard rules (violating any of these will block merging)

1. **No NVIDIA firmware assets, keys, or decrypted artifacts.** `.gitignore` already covers the common forms (`*.rom` / `*.bin` / `payload_*` etc.); firmware is locally regenerated from official inputs with pinned SHA-256 hashes — a mismatch is a hard stop, never a fallback to another version.
2. **The default build stays zero-write.** All GPU write paths live behind the `tx` feature gate (`--features tx` is built only for explicit transaction experiments); if your change introduces any PciIo write into the default build, explain why in the PR.
3. **Behavior claims need readback evidence.** Status codes, return values, and self-test output do not constitute proof of execution — readback comparison does (background in the [FAQ safety notes](docs/faq.en.md)). If you did not run on hardware, say so; do not extrapolate hardware behavior.
4. **License is GPL-2.0-only.** The reference implementations (gpu-unlock and friends) are GPL-2.0; declare your sources when borrowing behavior. Cross-licensed code is not accepted.
5. **No fabricated evidence.** Logs, hardware tests, reproduction steps, and successful verifications must be real; mark missing information as "unknown". The AI-drafting notes at the top of the issue templates apply equally to PRs.

### Conventions

- Runtime output strings are always English (existing convention); comments and docs are Chinese.
- Experiment artifacts are archived evidence-as-commits; `.gitattributes` marks evidence files `-text` for byte-exact cross-platform consistency.
- AI-assisted contributions are accepted, but the committer owns the content; disclosure follows the principle statement in the [README author section](README.en.md#about-the-author-and-team). Self-review especially for fabricated logs or verifications.
- Preserve existing code style and module boundaries; code blocks in `sim/` that mirror `loader/` carry "must stay textually identical" comments — when changing either side, update both and verify with `cargo xt`.

## Hardware experiments are not required

Work at the code and simulation layer (a green `cargo xt` + QEMU negative control) is fully sufficient for a good contribution — you are very welcome even without hardware. Conversely, if you own hardware and want to test on it, that is welcome even more: it feeds the [compatibility matrix](docs/compatibility.en.md) directly, via the [hardware report template](.github/ISSUE_TEMPLATE/02-hardware-report.yml).

Two reminders:

- When testing, follow the [recovery manual](docs/faq.en.md) and the [safety notes](docs/faq.en.md): readback verification, no same-session retry after a fault, power-cycle recovery.
- If your PR modifies the ignition transaction code itself, support it with simulation evidence (`cargo xt`) rather than fresh hardware results — write milestones follow a gating process ([AGENTS.md](AGENTS.md)), advanced by the maintainer and the hardware owner at their own pace.

## Boundaries for porting contributions

Please don't submit cross-chip ports done by "changing ID constants" — per-chip geometry (booter size, gadget addresses, signature sections) differs every family, and changing constants is not a verified port. A cross-family port means re-extraction plus re-audit; opening an issue to discuss first is very welcome.

## Code of Conduct

Please follow the [Code of Conduct](CODE_OF_CONDUCT.en.md) in issues, PRs, and discussions.
