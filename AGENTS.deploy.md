# Deployment agent guide

nv-uefi-unlocker is a UEFI pre-driver unlocker for GTX 16 and RTX 20/30/40 series. Deploy the existing tool for the user's GPU; do not turn deployment into firmware development. [AGENTS.md](AGENTS.md) governs development. Keep its safety constraints; its historical milestone/environment snapshots are not current deployment status.

## Read and decide

| Need | Source |
| --- | --- |
| Normal release/source workflow | [English README](README.en.md#get-started) |
| Device routing, evidence, exceptions | [Compatibility](docs/compatibility.en.md) |
| Required files and optional markers | [USB reference](docs/faq.en.md#usb) |
| Turing GSP prerequisites | [Turing](docs/faq.en.md#turing) |
| Linux RTD3 / hybrid display / eGPU | [RTD3](docs/faq.en.md#linux), [display](docs/faq.en.md#display), [Thunderbolt](docs/faq.en.md#thunderbolt) |
| In-OS read-back and recovery | [ssdiag](docs/faq.en.md#ssdiag), [recovery](docs/faq.en.md#recovery) |

1. Discover the **target** GPU model, PCI vendor:device, BDF, OS, driver, display mode, Secure Boot state and intended USB volume. On Windows use `nvidia-smi -q` and device/volume inventory; on Linux also use `lspci -nn` and `lsblk -f`. Distinguish the build host from the target. Ask only for facts you cannot discover. Never infer a USB drive letter/device or overwrite an unidentified volume.
2. Map the target using the compatibility document, not the model number alone. GTX 16/RTX 20 → `tu10x`; RTX 30 GA10x → `ga102`; RTX 40 → `ad102`. GA100, pre-Turing and Blackwell are unsupported. Record unresolved model/platform issues, particularly TU10x 0x91, before proposing a hardware attempt; do not claim an unverified card was tested.
3. For multi-GPU systems identify the intended card. Default loader behavior processes every supported GPU. To target one, write its actual BDF (`bb:dd.f` or `dddd:bb:dd.f`) to USB-root `tx-select-bdf.txt`; a missing/unparsable match aborts selection. Do not substitute another card.

## Prepare

- Prefer an extracted release from [Releases](https://github.com/physicsdolphin/nv-uefi-unlocker/releases). It contains `BOOTX64.EFI`, maker scripts and platform assets executables; NVIDIA firmware is regenerated locally. Keep generated firmware/payloads out of git.
- Target must boot in UEFI mode, with Secure Boot disabled or the EFI signed and trusted. Explain any required firmware setting to the user; a configuration check does not establish that the firmware accepted a change.
- **Windows Turing:** on the target, run `enable-turing-unlock.ps1` as administrator, then **Restart**. Verify `EnableGpuFirmware=1` and a non-N/A GSP firmware version in `nvidia-smi -q`. Hibernate resume does not apply the change. `-Remove` reverses the registry setting. **Linux Turing:** verify running GSP-RM; do not apply the Windows registry script.
- Linux: inspect effective `DynamicPowerManagement` and GPU power state; RTD3/D3cold clears the unlock. Follow the FAQ's distro-specific configuration and initramfs instructions only when applicable, then verify runtime state after reboot. Do not assume a saved config is effective.
- Prefer dGPU-direct and entry through Restart/one-time boot menu. D3cold/offline BARs require a different entry path, not blind MMIO or bypassing guards. Thunderbolt handoff can erase the state even when EFI succeeded.

## Generate assets

Run the maker on the TARGET machine with no family argument: it enumerates the machine's NVIDIA GPUs (display-class registry on Windows, `nvidia-smi`/`lspci` on Linux) and builds exactly the RTX 20/30/40 families found:

```powershell
.\make-usbroot.ps1
```

```bash
bash make-usbroot.sh
```

Only when the maker runs on a DIFFERENT machine than the target, pass the identified family explicitly (`-Family ad102|ga102|tu10x`, aliases `40`/`30`/`20`) — or `-Family all` / `--family all` to build all three families onto one stick.

Default output/work directories are `usbroot` / `inputs`. Windows needs PowerShell 5+; the prebuilt Linux assets binary needs glibc ≥ 2.34 (bash maker also requires bash 4+, GNU utilities, curl or wget). On Windows TLS failures, consult the FAQ's WSL/Git Bash alternative. A pinned input download/hash failure is a hard stop: do not replace versions or disable validation.

`-SkipGspCheck` / `--skip-gsp-check` is only for generating on a different machine; it does not waive target GSP prerequisites. Offline Turing proven-blob selection uses `-ProvenBlob` / `--proven-blob` with the pinned archive. Do not use these switches to force an unready target through the gate.

If building from source is needed, follow [Build](README.en.md#build-from-source-optional): `rustup target add x86_64-unknown-uefi`, `cargo build --release`, `cargo efi-tx`, `cargo xt`. Pass `target/release/assets[.exe]` and `target/x86_64-unknown-uefi/release/loader.efi` explicitly to the maker using `-AssetsExe`/`-BootEfi` or `--assets`/`--boot-efi`. `cargo efi` is read-only and cannot unlock. The ssdiag crate is separate: build it on Windows with `cargo build --release --manifest-path tools/ssdiag/Cargo.toml` if it is not obtained from a release.

## Assemble and check the USB

Use an identified FAT32 USB volume; preserve existing files and back up any destination `EFI/BOOT/BOOTX64.EFI`. Do not format a volume without explicit authorization. Generate into a local staging directory, not directly onto an existing boot medium: maker removes matching earlier assets from its output directory.

The maker output is **flat and contains intermediates**. Use its final `expected` list, not every `.bin` in the output. Required destination files (`F` = normalized family):

| Destination | Source / condition |
| --- | --- |
| `EFI/BOOT/BOOTX64.EFI` | generated `BOOTX64.EFI` (source build: maker copies `loader.efi` under this name) |
| USB root | `booter_load_F.bin`, `gsp_boot_desc_F.bin`, `gsp_boot_F.bin`, `fwimage_F.bin`, `fwsignature_F.bin`, `v67-F-four-plm.bin` |
| USB root, `ad102` / `ga102` | `scrubber_ad10x.bin` |
| USB root, `tu10x` | `sec2_bl_tu10x.bin`; preserve generated `gsp-gate.txt` as the build record |

Verify the destination set and family against maker output before calling media preparation complete. Never mix families within a single-family build; the only sanctioned multi-family form is an explicit `-Family all` / `--family all` stick shipped as the full expected union. Do not copy extraction intermediates as loader assets. Missing `v67-F-four-plm.bin` means replay, which still performs GPU writes/firmware execution/reset; it is not read-only diagnosis.

Do not write the system ESP, change NVRAM BootOrder, install a resident boot entry or format/repartition disks as part of the default workflow. Inspect existing markers before handoff; normal deployment must leave `i_know_what_i_am_doing.txt` absent (it enables automatic execution after 5 seconds). Back up an existing armed marker outside the USB root before removing it; retain armed mode only for an explicit unattended-operation request. Preserve normal manual confirmation. Advanced markers such as `no-reconnect.txt` need a matching FAQ symptom; do not blanket-enable workarounds.

## Handoff and acceptance

1. Give the user the exact target, prepared USB volume, one-time boot instructions and recovery route. Physical GPU execution requires explicit user intent; preparing media alone does not establish that the transaction ran.
2. Normal TX screen: **Space or Enter proceeds**; another key or 30-second timeout aborts to the bootloader without a GPU transaction. Target display can freeze/go black during GOP detach. Default chainload searches Windows first, then supported Linux loaders; custom `chain-target.txt` is documented in the FAQ.
3. After OS boot, preserve USB-root `loader.log` before another run. Inspect family-specific execution/selector read-backs and final handoff, not just a zero status or a generic all-green verdict. Ampere uses the PLM-flip verdict; do not impose Ada's marker/HALT criteria on it. Turing relocks PLM and skips FLR on the unlock path; a closed gate alone is not a failed unlock.
4. **Windows:** run `ssdiag-x86_64-windows.exe` (administrator if driver deployment is needed). **Linux:** from the checkout, run `sudo python3 tools/diag-unlock.py --bdf ACTUAL_TARGET_BDF`, substituting the inventoried BDF. Compare in-OS SS0/SS1 read-backs (`0x88888888` / low nibble `0x8`) and the family verdict. Do not reuse a physical BAR0 address from EFI or a previous boot.
5. If Ampere/Ada's gate is open but selectors reverted, explain the FAQ's `--reapply` write operation before using it; it cannot open a closed gate or restore a powered-off GPU. Keep initial diagnosis read-only. Turing remains read-only in ssdiag.
6. Compare the same workload, dimensions, precision, power settings and measurement method before/after. Report firmware execution, OS survival and performance separately. nvoc tuning is optional and requires its own appropriate settings; do not copy extreme benchmark voltages/power values onto another card.
7. STOP/fault/stuck black screen: **no retry in the same session**. Fully power off using the vendor's procedure, remove USB, boot normally and inspect driver/device state. Archive the failed log, then consult recovery/known issues. Do not turn a failed run into an automatic retry loop.

Final report: target identity; chosen release/build and family; prepared files/volume; steps requiring physical/user action; actual EFI read-back evidence; OS verdict and measurements if available; unresolved issues. If only media is prepared, say exactly that. Unlock state is volatile; cold boot/power loss requires replay. The current loader routes by family and logs PROM CRC32; per-card ROM/PROM SHA-256 enforcement remains pending, so do not claim it was performed by the loader.
