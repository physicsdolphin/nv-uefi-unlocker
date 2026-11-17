## What

Docs-only change (README + QUICKSTART). No code, no payload, no target-table change.

## Why

This repository's kernel module runs only on Linux, and the Windows section covers one captured, source-only port. The issue tracker keeps receiving questions a Linux module structurally cannot answer — users without a Linux environment, or users who want the transaction before any operating system starts (#89, #120, #131, #101). Reporters have already produced results with an independent UEFI application (#99, #132), but the README does not mention that path, so those questions have no anchor to point at. #82 and #117 adapt this repository's own Windows template rather than an independent tool, and #143 was produced with MindAllik/GPU-Tensor-Unlock-Universal, so none of those three is cited here as a data point.

## The change

Adds a short, contained section after the Latence.app section referencing [physicsdolphin/nv-uefi-unlocker](https://github.com/physicsdolphin/nv-uefi-unlocker) as an independent, OS-agnostic pre-driver UEFI unlocker: a removable-media UEFI application that performs the transaction before any OS starts and then chain-loads the machine's normal boot manager (Windows Boot Manager, GRUB, systemd-boot, ...), routing by chip family with its own per-device evidence tiers and limitations list. QUICKSTART gets a one-line pointer for readers who arrive without Linux.

Wording follows the existing containment convention (as with the Latence.app section and the third-party ports referenced in the FP8 paragraph): the link does not extend this module's supported targets, and coverage/evidence claims belong to that project's own matrix, not this one. The override it performs is volatile (replayed each boot, cleared by power loss), which the section states explicitly.

I am the author of that project; happy to adjust wording, placement, or trim it down to a single sentence if you prefer a smaller footprint.
