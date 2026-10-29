# nv-uefi-unlocker

[中文](README.md) | [English](README.en.md)

A UEFI pre-driver NVIDIA GPU unlocker (Windows, Rust). In the UEFI DXE
window **before** the NVIDIA driver loads, it performs the complete
firmware transaction: SEC2 scrubber → FWSEC-FRTS (WPR2) → signed Booter
booting GSP-RM → (gated) V67 four-write payload → read-back verification
→ FLR → chained handoff back to the Windows Boot Manager.

**Current status: three tracks closed end to end — 40-series (AD10x),
30-series (GA10x) and 20-series (TU10x); the 30 series is fully
compatible as of v2.1.0.** Support matrix:

- **RTX 20 series (Turing) ✅ verified (v0.2.0)** — RTX 2070 (TU106,
  `10de:1f07`) full-chain unlock: **FP32-accum GEMM 36 → 60-63 TFLOPS
  (+67-75%)**; 2060 Super (`10de:1f06`) community-verified on the same
  chain ([issue #12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12)).
  **Turing users must first run the [GSP firmware-mode
  script](#turing-20-series-prerequisite-gsp-firmware-mode-must-read) and
  RESTART** — see the Turing section below.
- **RTX 30 series (Ampere/GA10x) ✅ fully compatible (v2.1.0)** — since
  2026-10-28 the GA10x device-ID ranges (0x22xx/0x24xx/0x25xx) are fully
  routed: **the 30 series is fully compatible**. Evidence: RTX 3060 Ti
  LHR (GA104, `10de:2489`) unlocked with v2.1.0 ([issue
  #11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11):
  wait_plm verdict, PLM flip + SS0/SS1 read-backs all correct; passed on
  an AMD platform with a flashed cross-vendor VBIOS); the same chain is
  upstream-verified across the family (3060 / 2487, +58-85% with
  community tools). **30-series users do NOT need** the Turing
  GSP firmware-mode prerequisite.
  Additional EFI reports cover 3080 (`2206`), 3070 Laptop (`249d`) and 3080 Ti Laptop (`2420`); see [community reports](docs/community-reports.md).
- **RTX 40 series (Ada) ✅ verified** — RTX 4060 Laptop (`10de:28e0`,
  AD107M) four-write unlock, verified on both sides: firmware marker
  `0x67b00001` + the four PLM read-backs; Windows-side FP16 GEMM 29.89 →
  49.33 TFLOPS (1.65×). Full report:
  [docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md).

The mechanical differences between the three generations are in the
[compatibility matrix](#compatibility-matrix) (Turing's FEAT block lives
at 0x4096xx, FLR re-locks the PLMs, and the Windows driver requires GSP
mode — all different from Ada).

## Usage (RTX 40 series laptop/desktop, AD10x family)

### USB stick root layout (FAT32; produced by `make-usbroot.ps1`, or laid out by hand per this table)

| File | Purpose | Required |
| --- | --- | --- |
| `\EFI\BOOT\BOOTX64.EFI` | This tool (back up any existing file at this path first) | ✅ |
| `booter_load_ad102.bin` | AD102-family Booter bundle (BTL1) | ✅ |
| `gsp_boot_ad102.bin` + `gsp_boot_desc_ad102.bin` | GSP-RM boot segment + v5 descriptor | ✅ |
| `fwimage_ad102.bin` | GSP-RM firmware image (~80 MiB, radix3-loaded) | ✅ |
| `fwsignature_ad102.bin` | GSP-RM stock signature (referenced by WprMeta) | ✅ |
| `scrubber_ad10x.bin` | Kernel-identical SEC2 scrubber (BSI handoff 1→3) | ✅ |
| `v67-ad102-four-plm.bin` | **The V67 four-write payload — its presence = unlock mode** | optional |
| `tx-require-d0.txt` | Abort with zero writes when the entry state is not D0 (e.g. after hibernate resume) | optional |
| `tx-select-bdf.txt` | Multi-GPU card pin: contents `bb:dd.f` or `dddd:bb:dd.f` (hex) | optional |
| `i_know_what_i_am_doing.txt` | **Armed mode**: the confirm phase becomes a 5-second countdown that then runs automatically; any key during the countdown aborts (for unattended/automated use; the file's existence is the declaration of intent, its content is ignored) | optional |
| `chain-target.txt` | Chain-target pin: the first non-empty, non-`#` line = an EFI path tried first (e.g. `\EFI\limine\limine_x64.efi`), on every volume; if no volume has it, the tool falls back to the default order with a loud log line (issue #8 suggestion) | optional |
| `\loader.log` | (output) full stage log, the evidence record | output |

The table uses the Ada asset names; choose `ga102` for GA10x or `tu10x` for Turing in the maker. Put `BOOTX64.EFI` at `\EFI\BOOT\BOOTX64.EFI`; put the selected family's `.bin` files and optional marker files at the stick root. The current maker emits a flat output directory, including intermediate extraction files: copy the family's files listed by its final `expected` list, and create `EFI/BOOT/` for the executable. Do not copy every intermediate firmware image.

Without `v67-*.bin` the tool runs in **replay/diagnostic mode**: the full
chain without the unlock payload. It still detaches drivers, starts signed firmware and may reset the GPU; replay is a write transaction, not the read-only `cargo efi` build. The log
line `family 'xxx' transaction track not enabled` means that chip family
has no transaction track yet (all three of 20/30/40 are wired now; a card
with missing assets is skipped). For a first run on a card not yet in the
matrix, replay mode first: reaching `TX: execution verified` means the
whole path works — add the payload afterwards.

**Multi-GPU**: with several supported cards in the machine, the default is
to process **every one of them in firmware enumeration order** — a card
that is already unlocked (EALREADY) or unsupported yields to the next, and
a hard failure emergency-FLRs only that card without blocking the rest.
The display stays alive when the console lives on another card (only the
target card's GOP is detached). `\tx-select-bdf.txt` pins exactly one
card: a pin that matches nothing (or an unparsable file) refuses to guess
and runs zero transactions that boot (if the pinned card is absent, no
other card silently takes its place).

### Procedure

1. BIOS: disable Secure Boot. Pick the display mode / entry path from the
   measured table below — **dGPU-only (MUX) is recommended**; hybrid mode
   works only in the single entry path "restart, with the dGPU powered"
   (see the analysis for why).
2. Boot from the stick → version banner → **press Space to confirm**
   (Enter or any other key aborts; 30 s with no key automatically returns to Windows). If
   `i_know_what_i_am_doing.txt` exists in the stick root (armed mode),
   the confirm phase becomes a **5-second countdown that then runs
   automatically**; any key during the countdown aborts. The file itself
   is the explicit declaration of intent to run, for automation/unattended
   scenarios; the default (no file) is always "a human must press the key".
3. Everything after that is automatic: the tool chains back into bootmgfw
   and Windows whether the transaction succeeds or fails, and the full log
   lands in `\loader.log` on the stick. The loader lookup order is fixed:
   **bootmgfw → the distro shim/grub/systemd-boot/Limine loaders → the
   stick's generic `\EFI\BOOT\BOOTX64.EFI`**. Only that last tier — normally
   this very application — opens a 5-second window: any key returns to the
   firmware shell (the unlock is already complete and logged by then, and
   any OS can be started manually from the shell). Dual-boot users can put
   a custom EFI path first via `\chain-target.txt` (see the table above).
4. Replay once per cold boot (the unlock is volatile: nothing persisted,
   nothing flashed, cleared by power loss).

V67 payload and full asset regeneration: `cargo run -p assets --
generate-ad102` / `release/make-usbroot.ps1` (locally regenerated from
SHA-256-pinned 610.43.03 public inputs, every artifact hash hard-verified;
expected values in the report hash tables).

The release package ships only scripts and self-built binaries
(`BOOTX64.EFI`, `assets-x86_64-windows.exe`, `assets-x86_64-linux`,
`make-usbroot.ps1`, `make-usbroot.sh`, `enable-turing-unlock.ps1`) and
**contains no NVIDIA bins** — every firmware file is regenerated on the
spot by the maker from the pinned SHA-256 public inputs above:

```powershell
.\make-usbroot.ps1 -Family ad102 ...   # 40 series (default; rtx40 / 40 are aliases)
.\make-usbroot.ps1 -Family ga102 ...   # 30 series (rtx30 / 30)
.\make-usbroot.ps1 -Family tu10x ...   # 20 / 16 series (rtx20 / 20; GSP gate enforced)
```

Linux/WSL/Git Bash equivalent: `./make-usbroot.sh`, with the same options
as the PowerShell script (`--family/--out/--work/--assets/--boot-efi/
--proven-blob/--skip-gsp-check`). The release package contains the Linux
pipeline binary `assets-x86_64-linux` (glibc >= 2.34, Ubuntu 22.04+ /
Debian 12+ and newer), which the script prefers and chmod +x's on the fly
(zips carry no Unix exec bits — run the script itself via
`bash make-usbroot.sh` or chmod it first); inside a repo checkout without
it the script falls back to an automatic
`cargo build -p assets` (Rust toolchain needed once). The tu10x GSP gate on
Linux reads the nvidia-smi runtime state (the proprietary driver 545+ runs
GSP-RM by default on Turing and newer — no EnableGpuFirmware-style switch
exists or is needed; the gate catches "driver too old" and "GSP force-disabled
via modprobe config"), and it probes the host's RTD3 state with an issue #9 notice.

Turing's proven BL archive `gsp_rm_boot_dbg.bin` is downloaded
automatically by the script from the SHA-256-pinned cmp50hx repository;
when offline, pass `-ProvenBlob <path>` with a local copy.

### Display mode × previous exit: measured availability (G614JV / 4060L / all three display modes covered, 2026-10-26)

| Previous exit | dGPU at this boot | This tool |
| --- | --- | --- |
| hybrid mode, iGPU-only shutdown | cold: no GOP, BAR0 all `0xffffffff` | ❌ claim preflight reports "dGPU powered off (D3cold)" |
| hybrid mode, hybrid shutdown → restart | hot: GOP present, registers readable | ✅ full chain works (four writes + selectors surviving FLR — the success log in evidence) |
| hybrid mode, after hibernate/wake, D3-gated | parked in D3, domains gated (VGA_WORKSPACE reads `0xbadf1301`) | ❌ claim preflight rejects it |
| hybrid mode, after hibernate/wake, transaction runs | registers readable, whole chain can go green — but the card is warm | ⚠️ a green chain is not a live unlock: it can be invalidated after handoff; read `HANDOFF-OUT` and probe from Windows (`docs/warm-resume-entry.md`) |
| hybrid mode, after hibernate/wake, **TU106 measured** | registers readable, full chain green | ✅ **unlock written and verified in Windows** (2026-10-27, 2070 desktop: hibernate resume → stick → full chain → unlock alive into Windows). The "invalidation" warnings in the two rows above did not reproduce on the TU106 + GSP-mode combination |
| dGPU-only, after hibernate or restart | always the panel owner, always powered | ✅ both work |

Operational advice: **choose "Restart" from Windows** (or cold-boot
straight into the stick from the boot menu) before running; do not use the
"shut down then power on" iGPU-only path, and do not enter the stick right
after a hibernate wake. The PM preflight is implemented: a non-D0 entry
logs `PM: WARM ENTRY` and pulls the card back to D0 (read-back confirmed);
for stricter behaviour drop an empty `\tx-require-d0.txt` in the stick
root — a warm entry then aborts immediately with zero writes and zero
resets.

**Why (it is not about which mode is better; it is whom the firmware
unconditionally POSTs)**

The firmware unconditionally runs VBIOS POST/devinit only on the GPU it
needs for its own display:

- With dGPU-only, the dGPU owns the panel. Every path — cold boot, warm
  restart, post-S3/S4 resume — must re-initialise it, so the dGPU stays
  powered, keeps its GOP and its register domains stay readable. That is
  why both hibernate and restart work there.
- In hybrid mode the iGPU can carry the display, so the firmware only does
  a best-effort job on the dGPU: cold boot and hybrid restart do POST it
  (hence "hybrid shutdown → restart" works — exactly the state of that
  4060 success log), but
  - an iGPU-only shutdown lets the EC cut dGPU power before POST (D3cold),
    BAR0 all-ones;
  - hibernate goes through the resume path, not POST: the firmware does
    not re-run devinit, the EC keeps the dGPU in D3 and the PRI domains
    stay gated — VGA_WORKSPACE reading `0xbadf1301` is that gate projected
    into a register.
- This tool works before any driver and has no ACPI interpreter, so it
  cannot run `_PS0`/PR3 the way the Windows driver does. Those three
  failing states can only be avoided by choosing another entry path —
  they cannot be fixed in code.

## Linux note: runtime D3 (RTD3) erases the unlock after handoff (issue #9)

The unlock is a volatile overlay (nothing persists by design; cleared by
power loss), and **D3cold is a power loss for the affected power domains**.
If the Linux NVIDIA driver runs with runtime D3 (RTD3) enabled, the GPU is
runtime-suspended into D3cold as soon as it idles — and the unlock state
goes with it. Symptom: `loader.log` is fully green (marker/PLM read-backs
all correct) but performance is back to stock inside the OS
([issue #9](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/9),
4050 Laptop `28a1` + Ubuntu: GPU parked in D3cold/suspended with the
effective `DynamicPowerManagement` at `0x02`).

Ubuntu and derivatives enable RTD3 by default on supported laptops (the
distro file `/usr/lib/modprobe.d/nvidia-runtimepm.conf` pins
`options nvidia NVreg_DynamicPowerManagement=0x02`). Disabling it has
three pitfalls; miss one and the change is a no-op:

1. **Reuse the distro file's basename.** modprobe.d rule: for same-named
   files the first one read wins and same-named files in later directories
   are **ignored entirely** (modprobe.d(5)); options from *differently*
   named files merely accumulate and empirically lose to the distro value
   (issue #9: a differently named `nvidia-pm.conf` with `0x00` left
   `modprobe -c` showing both lines and the driver still running `2`).
   Reusing the basename shadows the distro file wholesale:

   ```bash
   echo 'options nvidia NVreg_DynamicPowerManagement=0x00' | \
       sudo tee /etc/modprobe.d/nvidia-runtimepm.conf
   ```

2. **Rebuild the initramfs** — the NVIDIA module is loaded early from the
   initramfs, so a config that never enters the image is a no-op:

   ```bash
   sudo update-initramfs -u      # Debian/Ubuntu (Fedora/RHEL: sudo dracut -f)
   ```

3. **After reboot, verify the effective value, not the file contents**
   (same principle as this project's constitution, rule 1):

   ```bash
   grep DynamicPowerManagement /proc/driver/nvidia/params   # want 0x00
   modprobe -c | grep -i DynamicPowerManagement             # exactly one 0x00 line
   ```

Quick triage — "log all green but no gain in the OS": walk this table
before filing an issue:

| Check | Command | Unlock alive looks like |
| --- | --- | --- |
| Effective driver param | `grep DynamicPowerManagement /proc/driver/nvidia/params` | `0x00` (`0x02/0x03` = RTD3 on) |
| Merged modprobe config | `modprobe -c \| grep -i DynamicPowerManagement` | exactly one line, `0x00` |
| GPU power state | `cat /sys/bus/pci/devices/<bdf>/power_state` | `D0` (`D3cold` = powered off, unlock gone) |
| Suspension corroboration | `nvidia-smi` | normal readouts (all N/A while suspended) |

Same semantics apply to S3/S4 suspend/resume (power loss — just re-run the
USB transaction once). D3cold is the nastiest one: it errors nothing,
blacks out nothing — it quietly wipes the unlock. The Linux maker
(`make-usbroot.sh`) probes the host's RTD3 state while building the stick
and prints a notice.

## Thunderbolt / hot-plug root port eGPU: a volatile-unlock-hostile environment (known limitation)

User report (2026-10-28): with a Thunderbolt dock, the eGPU unlock ran
green end to end in EFI, but the unlock was gone after entering Windows.
Mechanically this is an environment issue, not a transaction issue:

- the EFI unlock works because firmware established the PCIe tunnel at
  POST, so the eGPU is visible to DXE;
- entering Windows, the TB connection manager (the firmware ICM / the
  Windows TB stack's approval and tunnel re-establishment) can **tear down
  and re-establish the tunnel** — to the eGPU that is equivalent to a hot
  reset / PERST#; dock power management (TB controller D3cold, S0ix,
  enclosure power policies) may cut power outright. The unlock is a
  volatile overlay: any conventional reset (hot reset/SBR, PERST#) or
  power loss erases it — same family as issue #9's RTD3/D3cold.

Reference: cmp170hx measured Thunderbolt 3 eGPUs failing the kmod unlock
itself (`Booter Load 0x15`); Oculink works (it is an extension cable in
essence). **The supported environment is native PCIe slots / dGPU-direct
laptops.** TB / hot-plug-root-port users: treat "no gain after entering
the OS" as expected behavior — no need to file an issue.

**TB field-test call (2026-10-29)**: `tools/ssdiag` turns this scenario
into a cheap experiment — please run it and report the data (Discussions
or a new issue, full output attached):

1. Unlock via the stick → in Windows, run `ssdiag` for the verdict.
   **Gate open + stock values** (SS0/SS1 not 0x88888888/0x8) → the values
   were re-derived; `ssdiag --reapply` can write them back without a
   reboot. **Gate closed** (reads come back badf) → the tunnel rebuild
   performed a real reset; only the USB stick helps, and a reapply is
   meaningless in that state.
2. On a "gate open" verdict, run `ssdiag --reapply` and measure the same
   GEMM load before/after (cuBLAS/GPU-Z both fine).
3. Paste both `ssdiag` outputs and the throughput pair. Success or
   failure, every report is a data point: for the first time this
   distinguishes "the tunnel rebuild only re-derived the values" from
   "it performed a real reset" — exactly what we could not tell before,
   which is why the whole scenario was filed as a blanket known
   limitation.

Boundaries unchanged: after dock sleep / power cut (D3cold) a reapply is
meaningless (the card lost power; re-powering goes through PERST# and
re-erases); a pure link retrain (below) does not touch the unlock and
needs no reapply.

One PCIe-semantics clarification worth keeping: **a pure link retrain
(Retrain Link / a TLS change / the LTSSM re-run triggered by ASPM config)
does not reset the endpoint** — device registers and config space are
preserved and the unlock is untouched. What erases the unlock is the
conventional-reset family (hot reset/SBR, PERST#, the FLR re-lock on
Turing) and power loss (D3cold/S3). The kernel does not retrain links
gratuitously during a normal boot; the real x86 "kernel touches the link"
scenarios are AER/DPC error recovery (only after errors — exactly what the
Turing GSP-mode gate cuts off at the source) and hot-plug re-scan — the
latter being this section's Thunderbolt case.

## Combining with the nvoc kmd power-wall unlock

This tool is **compatible and orthogonal** with the kmd power-wall unlock
of **[Skyworks-Neo/nvoc](https://github.com/Skyworks-Neo/nvoc)** (a
project led by [the author](https://github.com/physicsdolphin)): this tool
removes the tensor issue-rate clamp in the UEFI pre-driver window
(FEAT/WPR/WPRCFG PLM), while the nvoc kmd removes the power wall inside
the OS (currently applies to mobile GPUs (measured working) and 10/20-series
desktop GPUs (the driver read-back is valid but the effect is untested —
every such card I hold is shunt-modded to "0 W")); `nvoc
set-volt-rail-limit` lifts the slot 0/1/2 Target Wall, the VBIOS Wall and
the VRM Max Wall (Vop) voltage walls (applies to every GPU from the 10
series on) — the two act at different layers ("below" the driver + "above"
the driver), they stack, and combining them is recommended for extreme
performance.

<img width="1280" height="595" alt="image" src="https://github.com/user-attachments/assets/1f60b619-b367-4823-9764-1caa44575501" />

### Measured extremes (RTX 4060 Laptop 28e0, FP16 GEMM + nvoc)

> All FP-precision throughput figures in this document were measured with
> **[cli-stressor-cuda-rs](https://github.com/Skyworks-Neo/nvoc/tree/main/cli-stressor-cuda-rs)**
> from [Skyworks-Neo/nvoc](https://github.com/Skyworks-Neo/nvoc).

| Workload | Value |
| --- | --- |
| FP16 GEMM 4096 (this tool only) | 49.0–49.3 TFLOPS (stock 29.89, 1.65×) |
| FP16 GEMM 8192 (+nvoc power wall) | **60.8–63.1 TFLOPS** (~**2.1×** vs stock) |
| Core clock peak | **3045 MHz** @ 1.200 V |
| Board power peak | **206.8 W** (chip ~163.7 W + VRAM 21.8 W + PWR_SRC 24.8 W; 16-pin 164.4 W @ 19.8 V) |
| Temperatures | Hot Spot peak **101.8–104.4 °C**, core 79–81 °C, VRAM 58–66 °C |
| Limiter | PerCap Reason alternates between **Pwr** (power wall) and **Thrm** (thermal wall) |

Interpretation: after the unlock the card at full load first hits the
power wall (Pwr); with the wall lifted by nvoc it switches to the thermal
wall (Thrm) — **3045 MHz / 206.8 W / 104 °C Hot Spot is the physical
limit band of this 8 GB 4060 Laptop**; beyond that, any further gain can
only come from better cooling.

### What the unlock actually is: disabled transistors back to work, not overclocking

The extra compute cannot be had for free — silicon still obeys its
electrical and thermodynamic laws.

The clamped SM issue rate is a **tensor datapath that NVIDIA deliberately
switched off in firmware with efuses at the software level**
(manufacturing segmentation), not a downclock or a power limit — clocks,
voltage and the power wall are all unrelated to it. Controlled experiment
(RTX 4060 Laptop, ASUS ROG Strix G16 **G614JV**, FP16 tensor, same card,
same precision, the only two variables being the issue rate and the power
wall):

| State | Clock | Chip power | Wall hit | FP16 tensor |
| --- | --- | --- | --- | --- |
| stock | **2.52 GHz** | 95–105 W | **voltage wall** | 30.5 TFLOPS |
| SM issue-rate unlock only | **2.37 GHz** | 130–140 W | **hits the power wall** | 47.50 TFLOPS |
| power unlock only (+nvoc voltage-wall lift + extreme OC) | **3.00 GHz** | 155–160 W | **no power wall hit** | 35.9 TFLOPS |
| SM issue rate + power unlocked (+nvoc voltage-wall lift + extreme OC) | avg **2.89 GHz** | 190–208 W | thermal wall + power-delivery limit | **64.30 TFLOPS** |

After the issue-rate unlock the frequency **drops 3.7%** and power rises
34%, yet throughput rises **79%** — not one of the extra FLOPs can come
from clocks or power delivery; they can only come from **the full
transistor set no longer idling**: once the segmentation mask lets them
through, the suspended slots that consumed a NOP on every clock edge load
real instructions again and the participating tensor-core count returns
to the full design set — the power increase is idling becoming work. That
is also why pure overclocking/over-voltage/power-wall lifting can never
close the gap (stock, overclocked to 3.0 GHz and hitting no wall at all,
only reaches 35.9 T), while the unlock doubles throughput at a *lower*
clock.

> **⚠️ Hardware risk warning**
> Stacking both unlocks pushes chip power and heat density **far outside
> the factory design envelope** (the table above is the measured extreme:
> 200+ W board power, sustained Hot Spot around 105 °C). Before use you
> must verify and continuously monitor:
> - **Cooling**: paste/liquid-metal condition, fan curves, vapor-chamber
>   and heat-pipe spec — a Hot-Spot-to-core delta >20 °C means the
>   interface material is at its limit;
> - **Power delivery**: adapter rating, VRM headroom, 16-pin/power-chain
>   temperature rise (measured 16-pin 164 W @ 19.8 V);
> - **Monitoring**: keep GPU-Z/HWiNFO Board Power, Hot Spot and PerCap
>   Reason on screen at all times; a Hot Spot sustained >100 °C is
>   running bare.
>
> **No liability is accepted for physical damage to the chip, power
> delivery or the machine caused by over-power, over-voltage or
> over-temperature** — the unlock is volatile, but a fried die does not
> recover.

<img width="287" height="520" alt="image" src="https://github.com/user-attachments/assets/d5e5fb50-4504-4b76-ba43-56e5db078b8b" />

## How it works (overview)

NVIDIA's compute clamp is a hardware gate on the PRI bus (the PLM, default
`0xffffff8f` = writable only by heavy-secure firmware); host/driver writes
to protected registers are silently dropped. The only way through is to
have **the signed firmware itself** write from the privileged side — this
tool replicates the kernel driver's complete boot chain in the UEFI DXE
window (the only equivalent pre-driver window), and injects the
community-reversed V67 payload at the "hand the Booter its signature"
step:

```
claim(detach GOP/drivers + exclusive) → scrubber(BSI handoff 1→3)
→ FWSEC-FRTS(WPR2) → booter(correct signature + BROM verify, mailbox=WprMeta)
→ [V67: unbounded signature DMA overflows the DMEM stack → ROP →
FEAT/WPR/WPRCFG/FBPA four writes + marker 0x67b00001]
→ selector writes (host writes stick once the PLM is open) → signature
restore → FLR (cold-card handback) → bootmgfw
```

Every step is verified by read-back (marker + HALT + per-register PLM +
AON survival after FLR); on any failure the tool refuses and FLRs. Nothing
persists, nothing is flashed, everything clears on power loss. Details:
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md).

## Compatibility matrix

Judgement basis: this tool routes by **chip family** (all IDs of a family
share the same Booter/payload assets, matching the upstream
`ad102_family()`/GA10x routing). Tier meanings: ✅ verified here (full
chain + payload on real silicon, including attributed community reports); 🟢 upstream silicon-verified (same
family proven on the kmod side, our EFI path is structurally identical —
expected compatible); 🟡 path exists, unverified (code ready, no silicon
evidence); ❌ incompatible.

**Since 2026-10-28 the routing is open across the full NVIDIA driver
list** for the three transaction families (open-gpu-kernel-modules README
"Compatible GPUs", 217 IDs): `0x1e02..=0x1fff` and `0x2182..=0x21d1`
(Turing); `0x2200..=0x22ff` and `0x2400..=0x25ff` (Ampere; 0x23xx =
Hopper, deliberately absent); `0x2600..=0x28ff` (Ada). Any same-family
ID (mobile / workstation / CMP / modded-card) runs without a per-ID
table entry; unlisted IDs inherit the evidence tier of their family row.
**GA100 (A100 `0x20b0`/`0x20c2`, A30 `0x20f1`) is not GA10x and has been
removed from the ga102 routing** (a latent cross-family mismatch);
Blackwell (0x2bxx+) is out of scope.

| Family | Cards (device IDs) | Tier |
| --- | --- | --- |
| AD107 | 4060 Laptop **28e0** | ✅ verified (this report) |
| AD107 | 4060 2882 / 4060 Laptop 28a0 / 4050M 28e1 | 🟢 upstream silicon-verified (28e1 has a 2.00× measured report) |
| AD107 | 4050 Laptop **28a1** | ✅ verified (2026-10-26, v0.1.3): full chain on silicon (WPR2 verified → booter marker → four writes → selectors → signature restore → AON survival after FLR); FP16 tensor **22 → 38-42 TFLOPS (1.7-1.9×)**, integrity self-tests green; the gated `0xbadf1301` goes through the fallback |
| AD106 | 4060Ti 2803 / 4060Ti 16G 2805 | **2803 ✅ verified** (PR #5, AD106 desktop: replay + unlock + four writes read back, FP16 46.78→75.28 TFLOPS); **2805 ✅ community-verified** ([report](docs/evidence/ad106-2805-nuc13/report.md), ASUS 1043:8921 + Intel NUC13RNGi9, v2.1.0): FP16 45.78→75.24, BF16 44.84→68.80 TFLOPS; FP32 control unchanged, 75 s load test |
| AD106 | 4070 Laptop **2820** (same ID on modded cards) | 🟡 routed (2026-10-28): shares the AD106 family assets, same die family as 2803 (upstream's table lacks this ID too). First silicon run: post the full `\loader.log` per "When it does not work / misbehaves" below; the tier moves up only after that. **Modded cards: also include the original card model and VBIOS version** |
| AD104 | 4070Ti 2782 / 4070S 2783 / 4070 2786 | **2783 ✅ community-verified** ([issue #3](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/3), v0.1.4, Palit RTX 4070 SUPER + MSI B760 TOMAHAWK WIFI, 220 W): replay/unlock read-backs and FLR survival; 8192 GEMM FP16 70.5→100.1, BF16 71.0→107.0 TFLOPS. Other listed IDs retain upstream evidence |
| AD103 | 4080S 2702 / 4080 2704 / 4070TiS 2705 / **4070 2709** / 4090M 2757 | **2709 ✅ verified** ([issue #8](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/8), B760 desktop dGPU-only: full four-write chain read back, selectors surviving FLR; OS-side sm_89 mma microbenchmark FP16 ~61→**123.91**, BF16 **118.90**, FP8 **243.55** TFLOPS — all **2.0×**); the rest 🟢 upstream silicon-verified |
| AD102 | 4090 2684 / 4090D 2685 / 4070TiS 2689 | 🟢 upstream silicon-verified |
| GA10x | 3090/3080 2204/2206, 3070s 2484–248c, 3060s 2501–2584 etc. (0x22xx/0x24xx/0x25xx fully open since 2026-10-28; the former 20c2/20f1 are GA100-class and have been removed) | **3060 Ti LHR 2489 ✅ verified** ([issue #11](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/11), 2026-10-28, v2.1.0, @hunan-NRT): GA102-asset V67 chain passed the wait_plm verdict (PLM flip 0xffffff8f→0xffffffff read back, ~149 ms; SS0/SS1 written and read back 0x88888888/0x8, FEAT_READOUT_1=0) → clean Windows handoff; **AMD platform** (B450M Mortar + 5700X3D); card runs a flashed Inno3D iChill 3060 Ti VBIOS (cross-VBIOS is fine, FWSEC PROM reads normally). Additional EFI reports: **2206 / 249d / 2420 ✅ community-verified** (v2.1.0; [results and platform limits](docs/community-reports.md)). Other same-family cards remain compatible; these are the individually documented EFI results (success-verdict fix traced in [issue #6](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/6)) |
| **TU106** | **RTX 2070 1f07 / 2060 Super 1f06** | **1f07 ✅ verified** (2026-10-27, v0.2.0): full chain (FWSEC ignite → BOOT_DIRECT booter → V67 fire → PLM 0x409650 fully open → SS0/SS1 written+read back → signature restore); **FP32-accum 36 → 60-63 TFLOPS (+67-75%)**. **Prerequisite: Windows-side GSP firmware mode (section below), RESTART to apply**. Measured host: Intel 10980HK ES (QTJ1) + 2070 hybrid-output desktop platform; **hibernate-resume → stick unlock write → Windows verification passed**. **1f06 ✅ verified** ([issue #12](https://github.com/physicsdolphin/nv-uefi-unlocker/issues/12), 2026-10-27, v0.2.0, @vadash): full chain (FWSEC → WPR2 verified → BOOT_DIRECT booter → PLM fully open → SS0/SS1 written+read back 0x88888888/0x8 → signature restore → unlock values persist through the PLM relock → FLR skipped per semantics) → clean Windows handoff; **AMD platform** (AM4/Promontory, the bridge-BME fix path exercised); 8 GB SKU shares the 8192 MiB geometry table with the 2070 (other TU106 capacities need the table extended first) |
| TU102/TU104/TU116 (rest of the 20/16 series) | 1e02–1fff, 2182–21d1 etc. | 🟢 same-family assets + community silicon proof (2080Ti 22G chenxluo +51-75%, 2080Ti 11G nlzy/haohervchb ~1.6×, same chain) |
| Older than Turing (Pascal…) | — | ❌ no transaction track |
| Blackwell (50 series) / GA100 | — | ❌ different chain, out of scope |

## Turing (20 series) prerequisite: GSP firmware mode (must read)

Turing's Windows driver **accepts the pre-OS unlock state only in GSP-RM
firmware mode**; the legacy init path hitting an open FEAT PLM causes a
TDR storm / Code 43 (the 40-series/Ada has no such requirement — v0.1.x
40-series users are unaffected). **Run once** (admin PowerShell):

```powershell
tools\windows\enable-turing-unlock.ps1           # source tree; detects Turing cards and writes EnableGpuFirmware=1
.\enable-turing-unlock.ps1                       # inside the release package (next to the maker)
tools\windows\enable-turing-unlock.ps1 -Remove   # back to stock
```

The script enumerates `MatchingDeviceId` under
`HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-...}\` and writes
`EnableGpuFirmware=1` only for the Turing device-ID ranges (VEN_10DE,
0x1E02–0x1FFF and 0x2182–0x21D1).

**A Restart is required for it to take effect; do not rely on hibernate
resume**: resuming from hibernation restores the driver from the
hibernation image without re-initialising it — GSP mode does not engage,
and an old driver session hitting the unlock state TDRs. Flow: run the
script → Restart (or go straight from the restart into the stick unlock)
→ verify in Windows.

Turing sticks are regenerated on the spot by the maker (the release
package ships scripts only): `.\make-usbroot.ps1 -Family tu10x ...`. The
product has two more files than the 40-series stick: `sec2_bl_tu10x.bin`
(768 B, needed by the WITH_LOADER FWSEC) and `v67-tu10x-four-plm.bin`
(64000 B). For the tu10x family the maker **enforces** both
`EnableGpuFirmware=1` and the nvidia-smi GSP-firmware running state; if
either check fails the build is refused; add `-SkipGspCheck` to build on
another machine. After passing, the stick root gets a `gsp-gate.txt`
record. Measured TU106 factory clamp: the SS0 cold value is not the
default (0x15000/0x11000/0x0 drifting between boots) vs the unlock value
0x88888888 — the old "20 series has no factory segmentation" assumption
has been corrected.

**Why the trimmed BL is mandatory and hash-pinned**: the BOOT_DIRECT
booter consumes the first `0x6000` bytes of the GspRmBoot image
(`GSP_RM_BOOT_SIZE`, i.e. 24576 bytes, SHA-256 `de676bae...`) — exactly
the input of the silicon-proven v0.2.0 fire (log line
`gsp_boot_tu10x.bin loaded 24576/24576 bytes`). Neither the kernel
BINDATA 4 KiB v4 map nor the GSP ELF's first 0x6000 (`c76c43b1...`) is
it; the maker double-checks both the source archive and the output, and a
length or hash mismatch is a hard failure.

**After the Turing 2070 unlock — I forgot to take a "before" screenshot;
owners of the same card are welcome to run it themselves and see for
themselves:**

<img width="800" height="450" alt="image" src="https://github.com/user-attachments/assets/8c103363-8e44-4d46-ab47-a6c063ae1e6f" />

**When it does not work / misbehaves**: open an
[Issue](https://github.com/physicsdolphin/nv-uefi-unlocker/issues) with
the full `\loader.log` from the stick root (the log carries the complete
stage trace and is the only thing we can diagnose from; strip
machine-specific details first if you like). Success/failure data points
(card model + key log lines + verdict values) are welcome in
[Discussions](https://github.com/physicsdolphin/nv-uefi-unlocker/discussions)
to extend the matrix.

**Linux users**: if the symptom is "the `loader.log` is all green but
there is no gain inside the OS", walk the triage table in the
"Linux note: runtime D3" section above first — RTD3/D3cold silently wipes
the volatile unlock after handoff (issue #9). That is not a transaction
failure; do not rush to file the log.

### Known issue (fixed in the loader): booter status 0x89 = Bus Master not enabled on the GPU's upstream PCI bridge

**Symptom**: the log reaches `POST-BOOT: booter status 0x89 (nonzero =
failed)` and the transaction aborts with EIO. Codes `0x89`-`0x90` belong
to the booter's WPR-descriptor validation (`wpr_desc_validate`): the
256-byte WprMeta the booter DMA-read back is not what was written.

**Root cause** (PR #5, found and fixed by
[@8bit2qubit](https://github.com/8bit2qubit)): the booter stage is the
transaction's **first device-initiated host DMA** (everything before it —
scrubber/FWSEC — uses portal copies and PROM reads, no device DMA).
Desktop firmware typically leaves **Bus Master disabled on the GPU's
upstream PCI bridge** during boot services (the firmware never DMA'd
through the GPU this boot — GOP display works through CPU-side MMIO,
while the Bus Master bit only gates **device-initiated** bus
transactions). The GPU's DMA requests are terminated at the root port: the
GPU's PCI status bit13 `Received Master Abort` sets, the read returns
all-1s, and the descriptor check fails. This depends only on the bridge's
Bus Master bit, not on the chipset vendor: reproduced on H310 / B760 /
B850 desktops.

**Fix**: built into the loader (PR #5 merged) — before firing the booter,
the loader enables Bus Master on every PCI bridge along the GPU's upstream
path that does not have it set, verifies by readback, and restores the
original state when the transaction ends; the log keeps `DMA-PATH` /
`BRIDGE-BME` lines. Single-variable verified on an AD106 4060 Ti desktop
(2803): 0x89 before the fix → after the fix, replay `MAILBOX0=0`, unlock
marker + all four writes read back verified, FP16 46.78 → **75.28**
TFLOPS (`docs/evidence/ad106-4060ti-desktop/`; full mechanism in
`docs/bridge-bme-root-cause.md`).

**Historical correction**: this section previously attributed 0x89 to
firmware IOMMU (AMD-Vi) / TSME translation — disproven. The root cause is
the bridge Bus Master bit; the "stage everything in VRAM to bypass
platform DMA" fallback was also disproven by experiment (the booter's DMA
goes through its own virtual address mapping, identity-mapped to host
sysmem; switching the FBIF TARGET has no effect), see
`docs/bridge-bme-root-cause.md`.

**Verified harmless** (no need to disable):

- **SVM (AMD-V)**: R9000P 7945HX with SVM on (Task Manager
  "Virtualization: Enabled") unlocks normally;
- **Intel VT-x (virtualization) and VT-d**: full chain verified green
  with both enabled on the reference machines.

> Terminology note: the Task Manager "Virtualization" field only reflects
> SVM/VT-x; the bridge Bus Master bit is PCI Command Register bit 2 and
> has nothing to do with "Virtualization".

### Known issue (fixed in the loader): AMI pathless GOP child resets the platform after a successful transaction

**Symptom**: the whole transaction is green (`MAILBOX0=0x67b00001`, PLM read-backs,
`POST-FLR: verified`), then the PC resets or hangs on a black screen before Windows starts. Releases
before v2.1.5 left `loader.log` at 0 bytes in this case, because the log was only flushed right
before the chain-load.

**Root cause** (PR #13, found on an ASUS ROG MAXIMUS Z790 APEX + RTX 4060 Ti 8 GB): this AMI firmware
layers a **GOP child with no device path** (a firmware splitter/console handle) on top of the NVIDIA
GOP. The detach removes the NVIDIA GOP, but the pathless child survives. A **recursive**
`ConnectController` then re-binds the console stack over that stale child, and the platform resets
inside the call. Two places did this: `reconnect_all` right after the post-FLR verdict, and the
`map -r`-style pass in `chain_load_bootmgfw`, which reconnected every PciIo handle including the GPU.

**Fix**: built into the loader (PR #13; node-aligned GOP ownership in `fe92339`). The reconnect stays
recursive by default (single-GOP boards are unchanged) and connects the vendor GOP only when a
pathless GOP survived the detach; the pre-chain pass skips NVIDIA display functions; `loader.log` is
flushed during the run. On an affected board the log shows
`RECONNECT: pathless GOP survived the detach - vendor GOP only (non-recursive)`, and `\no-reconnect.txt`
skips the reconnect entirely if a board still misbehaves. Evidence: `docs/evidence/ad106-asus-z790/`,
write-up `docs/ad106-asus-z790-windows-unlock.md`.

> The maintainer works on this **part-time** and may not see or answer
> issues immediately — the logs are kept and will be reviewed; patience
> appreciated.

## Repository layout

```
loader/    EFI application crate (no_std, x86_64-unknown-uefi, Rust + uefi-rs)
assets/    host-side asset pipeline (pinned-hash download/verify/extract/payload generation, pure Rust)
sim/       behavioral simulator + transaction state-machine tests (GPU-free development)
tools/     ssdiag: in-OS Windows unlock verdict / no-reboot repair (pmxdrv.sys embedded)
docs/      reports, milestone evidence (evidence-as-commits), register cold-value tables, open questions
```

## Safety constitution (copy, equivalent to AGENTS.md §1; violating any single item stops the work)

The loader currently records PROM CRC32 fingerprints. Per-card ROM/PROM SHA-256 pinning, required below, is pending; see [implementation status](docs/community-reports.md#使用说明与实现边界).

1. **Read-back is the only success criterion.** Status codes, return
   values, interrupts and self-test output never count as proof of
   execution.
2. **Never retry in the same session after a STOP/fault**; power off
   completely and resume through normal Windows boot.
3. Chain-load the ESP's `\EFI\Microsoft\Boot\bootmgfw.efi` **read-only**;
   never overwrite anything on the ESP; back up the stick's BOOTX64.EFI
   before replacing it.
4. No persistent boot entries: one-shot from USB/boot menu only; never
   touch NVRAM BootOrder.
5. Pin every card's identity (PCI vendor:device + ROM/PROM hash); refuse
   on card/ROM mismatch.
6. Never distribute or commit NVIDIA firmware; regenerate locally from
   pinned SHA-256 official inputs; keys/decrypted artifacts/payloads never
   enter the repository.
7. Gated writes: zero writes before M1; explicit user confirmation before
   M3; M4/M5 are separate projects.
8. Mock before hardware: transaction code must be fully green in the
   behavioral simulator first.
9. Secure Boot disabled or a self-signed cert in db (preferred).
10. License: GPL-2.0-only.

## Rollback manual

### Scenario A: black screen (chainload failure / mid-transaction abort)
No firmware configuration or ESP file was touched → force reboot → boot
Windows from the original boot entry and you are back. When the stick's
BOOTX64.EFI is not in the boot chain, the machine never enters our code
in the first place.

### Scenario B: STOP/fault mid-transaction
**Never retry in the same session** (firmware may still be running, DMA
buffers may still be owned) → power off completely (laptops: discharge
the EC per the vendor procedure, ASUS = unplug + hold power 40 s) →
remove the stick → boot Windows normally → check Device Manager / event
log for driver state → archive the evidence before considering the next
run.

### Scenario C: Windows driver misbehaves after handoff
Check Device Manager error codes; check nvlddmkm/Xid in the event log.
The unlock is a volatile overlay (gone on power loss) — one cold boot
returns the card to its factory state. If issues persist: another full
power cycle; last resort DDU + driver reinstall (this tool writes nothing
persistent; a misbehaving driver can only come from handoff timing, not
permanent damage).

## In-OS unlock verdict and repair on Windows: `tools/ssdiag`

Answers "is the unlock still live this boot?" without a reboot, and on
Ampere/Ada supports a **no-reboot repair**: when the FEAT gate is open but
the SS overrides were re-derived away by driver devinit (driver init,
sleep/resume, Thunderbolt rail re-enumeration), `--reapply` writes
SS0=0x88888888 / SS1=0x8 back and verifies by read-back (the
separate-lock method, issues #6/#19 lineage). On the Turing relock chain
the gate is closed inside Windows, so the tool stays read-only there —
but the Turing SS values persist for the whole Windows session, so the
in-OS read-back **is** a valid verdict; gate closed = re-run the stick.

```text
ssdiag                        # verdict: dump every NVIDIA display GPU
ssdiag --reapply              # repair: write the SS overrides back where the gate is open
ssdiag --select 2504          # multi-GPU: only cards whose DeviceID contains 2504
```

Capabilities and boundaries (v0.4.x):

- **BAR0 is always discovered from the OS** (physical assignments change
  every boot; a remembered base once hard-hung a machine, so stale
  addresses are refused). Multi-GPU: per-card dumps grouped by DeviceID,
  Display-class functions only (companion audio/USB-C excluded).
- The dump surface is the union of the registers the reference unlockers
  touch: the core/debug block (PMC_BOOT_0, PSTRAPS decoded, FB_SIZE_MIB,
  GFW, BSI_SCRATCH15, WPR2, MMU PLM + lock, GSP/SEC2, fuse rows), **FEAT
  clusters of both families** (Turing 0x4096xx incl. READOUT, Ampere/Ada
  0x823xxx), the Ada four-write targets (WPR/WPR_CFG/FBPA PLMs), the
  AD10x 64G-BAR1 line (CYA/CFG/XVE/cap mirror), the cmp170hx 26-register
  PLM survey, the Ampere FEAT sweep. Read-backs are badf-bucketed
  (PRIV_VIOLATION / UNDECODED / FLOORSWEPT) with colored verdicts and
  addresses (pipes/`NO_COLOR` degrade to plain text).
- Families Maxwell..Rubin recognized (incl. the GA10x
  `BOOT_0=0xb7xx00a1` quirk); the FEAT unlock verdict only where the
  cluster is mapped (Turing/Ampere/Ada).
- GPUs reading degenerate are woken on demand via the driver's own NVAPI
  GC6Control (GC6/GCOFF parking; field-proven on a 4060 Laptop).
- Driver safety: `pmxdrv.sys` (Intel-signed, hash-pinned at build time,
  re-verified at startup fail-closed) is **embedded in the exe**, deployed
  under its own service name `SSDIAG_PMXDRV`, and on every exit path the
  service is stopped + deleted and the temp file removed; services owned
  by other tools are never touched. If a PMXDRV device already exists
  (e.g. deployed by nvoc) it is borrowed with zero service changes. The
  deploy path needs admin (borrowing a running device works unelevated).
- Thunderbolt eGPUs: a reapply is only meaningful while the rail stays
  powered (see the Thunderbolt section above).

Get it: the release zip ships `ssdiag-x86_64-windows.exe` since v2.1.5;
source in `tools/ssdiag` (standalone crate, `cargo build --release` to
rebuild — see its `README.md`).

## Build

```
cargo build --release   # host crates (assets/sim) — the loader is not in default-members
cargo efi               # read-only EFI build (M0/M1 diagnostics)
cargo efi-tx            # gated transaction build (write paths; explicit use only)
cargo xt                # all host tests (assets + sim; see current test output for counts)
```

A bare `cargo build --workspace` tries to build the loader for the host
triple and fails — the loader only builds for `x86_64-unknown-uefi`; use
the aliases above.

## Community contributions and references

This project is a direct beneficiary and derivative of community work;
full credits in the same-named section of
[docs/ada28e0-uefi-report.md](docs/ada28e0-uefi-report.md). Core:

- [gpu-unlock](https://github.com/Zagitta/gpu-unlock) (GPL-2.0) — the
  transaction-chain porting source
- [gpu-unlock-ada](https://github.com/JustAResearcher/gpu-unlock-ada)
  (GPL-2.0) — the AD10x four-write chain origin; the 28e1 report
  (Windroid234) is this project's direct basis
- [cmp170hx](https://github.com/Consensus-Protocol/cmp170hx) — PLM
  semantics and verification methodology
- [cmp90hx-unlock](https://github.com/WildFlash1st/cmp90hx-unlock) — the
  V67 chain origin
- [AD103_win_unlocker](https://huggingface.co/TOTORONG/AD103_win_unlocker)
  (TOTORONG) — the Windows EFI carrier-craft precedent
- [open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules)
  (MIT) — first-hand authority on the boot path

License: **GPL-2.0-only** (`LICENSE`). NVIDIA firmware and VBIOS remain
NVIDIA property; regenerate them locally from pinned public packages
under their own license terms — they are not distributed in this
repository.

## Experimental TU102 22 GiB C-compatible profile

The opt-in `tu102-c-compat` feature ports the successful C first-attempt sequence
for the recorded 1e07/22 GiB/.e9 board. Both the C reference and this Rust candidate
have passed a hardware boot test; repeated cold boots and long-term stability
remain unverified. Existing transaction
tracks remain the default. See [the implementation notes](docs/tu102-c-compatible-fix.md).
Reproduce the separate payload with `cargo run -p assets -- generate-tu102-direct <output>`.
