# Security Policy

> 中文版：[SECURITY.md](SECURITY.md)

## Supported Versions

Security fixes target the latest release and the `main` branch only. There are no backports to older versions.

## How to Report a Vulnerability

**Please do not report security issues in public issues.**

Use GitHub private vulnerability reporting: the repository's **Security** tab → **Report a vulnerability**, or go directly to <https://github.com/physicsdolphin/nv-uefi-unlocker/security/advisories/new>.

When reporting, please include:

- The affected component (see the scope table below) and tool version / commit;
- Reproduction conditions, including the hardware platform (GPU model, motherboard, boot method);
- The observed behavior versus the expected behavior;
- Verifiable evidence (readback records, log excerpts).

Please do **not** attach NVIDIA firmware files to the report (this project does not host any firmware assets, by policy).

### Response Expectations

This project is maintained part-time, outside of full-time work; **there is no fixed response SLA**:

- Reports are acknowledged as soon as practical, typically within one to two weeks together with an initial assessment (impact, severity, remediation plan);
- Fix cadence depends on severity and real-life bandwidth; high-severity issues take priority;
- Disclosure is coordinated with the reporter before a fix ships; coordinated disclosure is the default;
- The reporter is credited after the fixed release ships (anonymously upon request).

If you have not heard back within about two weeks, please follow up on the same private report thread — it usually just means a notification was missed.

## Scope

The following are treated as security vulnerabilities: issues that **can lead to unintended GPU/memory write paths, supply-chain substitution, local privilege escalation, or bypassing the safety gates**:

| Component | Scope |
|---|---|
| `loader/` EFI application | BAR0 / PCI config-space write paths, FLR handling, DMA buffer lifetime, chainloading, effectiveness of the `tx` gate (is the default build truly zero-write) |
| `assets/` pipeline | Whether the pinned-input SHA-256 verification can be bypassed, out-of-bounds and path-escape in extraction, payload generation integrity |
| `tools/ssdiag` | The embedded `pmxdrv.sys` load surface (service creation / driver unload), BAR0 access boundaries, whether post-run cleanup can be subverted |
| `release/` packaging scripts | Command injection, path escape, or verification short-circuits in `make-usbroot.*` / `make-provenance.ps1` |

### Out of Scope

- **Vulnerabilities in NVIDIA VBIOS / GSP firmware themselves**: report those to NVIDIA PSIRT. This project studies unlock mechanisms; it does not reproduce or verify vendor firmware defects.
- **Usage side effects** (black screen, STOP, driver faults, hardware risk): see the [recovery manual](docs/faq.en.md) and the [safety notes](docs/faq.en.md) in the FAQ. Use the [bug report template](.github/ISSUE_TEMPLATE/01-bug-report.yml) for these, not this channel.
- **Compatibility issues and performance regressions**: see the [compatibility matrix](docs/compatibility.en.md); use the bug report template.
- Attack scenarios requiring **physical access**, a boot chain with **Secure Boot enabled without an enrolled self-signed certificate**, or **non-stock flashed firmware** — the threat model assumes the operator already has full control of the machine.

## Security Design Notes

The project's write-path discipline (the safety constitution, [AGENTS.md §1](AGENTS.md)) is directly relevant to what this policy covers:

1. **Readback is the only success criterion** — every write must be verified by readback; status codes, return values, and console output do not constitute proof of execution.
2. **Write operations are tier-gated** — the default build (no `tx` feature) performs zero writes; transactions that ignite firmware require an explicit build and the hardware owner's confirmation.
3. **Read-only chainloading** — no ESP file is ever overwritten, no persistent boot entry is created; entry is one-shot from a USB stick.
4. **Pinned supply chain** — NVIDIA firmware is locally regenerated from official inputs with pinned SHA-256 hashes; it is never redistributed or committed; a hash mismatch is a hard failure and never falls back to another version.

If you find that any of these rules can be bypassed (e.g. asset hash verification defeated, a write path reachable without the `tx` gate, ssdiag driver cleanup hijackable), **that is precisely a security vulnerability under this policy** — please use the private reporting channel.

## Related Documents

- [LICENSE](LICENSE) (GPL-2.0-only) and the copyright boundary statement in the [README](README.en.md)
- [docs/faq.en.md](docs/faq.en.md) (recovery manual and safety notes)
- [AGENTS.md](AGENTS.md) (development gating and experiment discipline) / [AGENTS.deploy.md](AGENTS.deploy.md) (deployment workflow)
