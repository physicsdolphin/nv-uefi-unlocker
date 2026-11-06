# TB4 eGPU RTX 4090 — the OS router takeover is the writer (2026-11-05)

Reporter `Ambolio`, desktop Thunderbolt 4 host, Razer Core X V2 enclosure,
`10de:2684` (AD102). An on-board RTX 5090 owns the console. This is the case
that closes the "tunnel rebuild / PERST# / enclosure power" suspects and makes
the OS-side USB4/Thunderbolt connection manager the **only** surviving writer —
the sixth arm of the canary table in the FAQ, and the only one where the reset
is performed by the OS rather than by a rail, a firmware reset or a driver.

The directory is named for the Linux arm that produced the proof; it also holds
the Windows suppression rounds (two hosts) and the clock/power sweep.

Earlier rounds on the same issue had already:

- disproven `DevPnPCapabilities=0x100` (Windows side, no effect);
- disabled `Usb4HostRouter` and the DeviceRouter **together** — the tunnel
  simply does not come up, so the card is absent, which isolates nothing;
- hot-added the card and run `--reapply` — `REFUSED: gate closed`;
- booted with the `nvlddmkm` service disabled — no driver, but also no readback
  (see Catnip's note below).

## The measurement

Both runs chain the loader into a live Linux (`chain-target.txt` → rEFInd →
SystemRescue 13.02, kernel 6.18.41) instead of Windows. No NVIDIA driver is
ever present; reads are root, sysfs `resource0` mmap, via
`tools/diag-unlock.py`.

**Run A — stock (kernel `thunderbolt` driver allowed).** At +3.7 s the module
registers and the kernel releases the enclosure's bus:
`pci_bus 0000:18: busn_res: [bus 18] is released`. At +4.3 s the Razer Core X
V2 re-appears as a *new* TB device; by measurement time the 4090 function is
gone from sysfs. The Linux USB4/TB router driver performs the same
takeover-and-remove that `Usb4HostRouter` performs on Windows.

The only readable GPU in Run A is the on-board 5090, which the loader never
armed: `FEAT_PLM=0xffffff8f` (closed), `CANARY_A/B = 0/0`. That is the canary's
**first untouched-card negative control** — it does not self-arm and does not
drift.

**Run B — `modprobe.blacklist=nouveau,thunderbolt`.** The 4090 stays on the bus
exactly as POST left it, `lsmod` empty, reads on `0000:18:00.0`:

| register | value |
|---|---|
| `FEAT_PLM` | `0xffffffff` (OPEN) |
| `FEAT_SS0` | `0x88888888` (full ×8) |
| `FEAT_SS1` | `0x00000008` (full) |
| `FEAT_READOUT_1` | `0x00000000` (full speed) |
| `CANARY_A/B` | `0x0164c0de` / `0x0165cafe` → intact |
| verdict | `UNLOCKED` |

The unlock survives the handoff, the POST-to-OS boot and living inside a booted
OS untouched. Nothing on the firmware, tunnel, POST or OS-entry path writes it;
the loss is exactly the router driver's takeover.

## Structural consequence (the catch-22 gets harder)

The takeover *is* the whole-domain reset, so the gate is closed **by the reset**,
not by `nvlddmkm` — it happens before any driver binds. Hence:

- the hot-add `REFUSED` and the service-disabled boot are one cause, not two;
- **there is no host-write path after takeover**: `--reapply` is structurally
  dead, and an in-OS readback only ever shows a closed gate;
- the only lever is to stop the takeover from performing the reset. Both OSes
  have a measured suppression: Linux `thunderbolt.host_reset=0`, Windows
  `Usb4DeviceRouter` `Start=4` with the host router left running. Disabling both
  routers together (the first Windows attempt) leaves no tunnel at all.

## Separation from #164

The `2484` X570 desktop case ([#164]) loses the FEAT block too, but the SS pair
is zeroed and the writer is undetermined. This host shares the same card class
of evidence (card fully excluded) but names the writer outright. The two must
not be merged: one is the OS connection manager, the other still has a
platform/firmware/driver candidate set.

## Method notes worth copying

- The ISO's own GRUB dropped to its rescue prompt when chainloaded (its embedded
  prefix resolves to the ISO's El Torito partition, not the stick). rEFInd
  (single self-contained EFI plus a two-line `refind.conf`, `timeout 2`) makes
  the chain unattended.
- The three initrds (Intel/AMD ucode + `sysresccd.img`) are concatenated into
  one cpio file.
- Secure Boot stayed **on** throughout; only GRUB's internal `shim_lock`
  objected to the unsigned rEFInd/kernel.

## Isolation limit on this host

Windows treats the screenless eGPU 4090 as the POST display adapter and forces
it up with the POST driver, so a "booted but no NVIDIA" Windows state is
unreachable here (and `PciIo RomImage` is empty because only the boot-VGA device
gets a shadow copy — the native-GOP branch cannot be tested on this topology
either). Chaining into Linux is the real alternative path, as Catnip noted:
`ssdiag` discovers BAR0 through OS resources, so a function that never got
resources has no window to map.

## Open

Answered below: the reset was suppressed on both OSes. Residual unknowns are
listed at the end.

[#164]: https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164

## Run C — `thunderbolt.host_reset=0` suppresses it (Linux, same host)

`modprobe.blacklist=nouveau thunderbolt.host_reset=0`, `thunderbolt` loaded
(`lsmod`: `thunderbolt 581632 0`), nouveau still blacklisted. The manager still
re-discovers the enclosure at +3.0 s (`new device found … Razer Core X V2`) but
`pci_bus 0000:18: busn_res: [bus 18] is released` **never appears**; at read
time `0000:18:00.0` is present with `driver=NONE power_state=D0` and the block
reads `PLM=0xffffffff`, `SS0=0x88888888`, `SS1=0x8`, `READOUT_1=0`, canary
intact, `UNLOCKED`.

So on this host the reset is the host-router init reset call, and it is a call
that can be skipped. This is the first state where the manager is loaded *and*
the unlock is live.

## Windows split-disable — the same result on two hosts

Two reporters independently set only `Usb4DeviceRouter`'s service `Start` to
`4` (Disabled) and left `Usb4HostRouter` at `3` (Manual); disabling both had
earlier been shown to leave no tunnel at all.

| host | router silicon | card | result |
|---|---|---|---|
| desktop TB4, Razer Core X V2 | ASMedia `1B21:2425` | AD102 `2684` | card enumerated, bound `nvlddmkm`, `nvidia-smi` OK, canary intact, `UNLOCKED` |
| Lenovo 83D4 laptop, Nano X dock (TB3) | Intel `8086:7EC2/7EC3`, `USB4_MS_CM` | GA104 `2484` | card enumerated, bound `nvlddmkm`, canary intact, `UNLOCKED`, 87.92 TF/s FP16 = 95% of the full-rate tensor figure, plus a locked-clock sweep |

Collateral on both: the enclosure's device-router services are gone (the
laptop's dock USB ports die, two `USB4\ROOT_DEVICE_ROUTER` nodes at `Problem
32` = service disabled, benign). GPU, card audio and `nvidia-smi` unaffected.
`ssdiag` 0.6.13 prints a `USB4:` block with both `Start` values so reports
carry their own configuration.

One confound to keep visible: the laptop arm also moved the Linux boot entry
behind Windows and cold-booted in the same run, so "`Usb4DeviceRouter` disabled
is the necessary item" is not yet single-variable. The natural control is
`Start=3` with everything else unchanged, expected to clear the canary.

## Control round — the causality closes both directions

`Start` back to `3`, everything else identical (same stick, chain →
`bootmgfw.efi`, cold F11 boot, same day): `CANARY_A/B = 0x00000000/0x00000000
-> cleared`, `SS0 = 0x01501012 SS1 = 0x00000006` (factory readouts),
`NOT_UNLOCKED`. Re-applying `Start=4` and cold-booting the same chain returns
`intact / SS0 = 0x88888888 / UNLOCKED`. Single variable flipped → re-lock
restored, and back again — the takeover reset is pinned to the device-router
driver's binding, not to the tunnel coming up.

Answers recorded with it: the card in the `Start=4` cell is fully functional
(`nvlddmkm` `Problem 0`, days of real CUDA compute, a monitor on the card as
primary incl. its DP/HDMI audio); `Start=4` persisted every boot over days —
the only thing that ever changed it was `sc config` itself (a driver-update
re-arm is untested and labelled as such). The FAQ line this produces: one-time
service disable + per-session chain boot (the value persists, the state does
not — a plain cold boot re-locks).

## Undock / redock — hot re-plug does not come back

Machine in the `Start=4` cell, live-tested: cable pulled and re-inserted
mid-session → the card sits at `Code 45` ("currently not connected"), the dock
router nodes likewise Unknown; `pnputil /scan-devices` (elevated) completed a
full scan with no change; a dock power cycle left the tunnel down. Recovery =
reboot through the loader chain — and after that reboot the canary is `intact`
and the verdict `UNLOCKED`. The conservative "plug in before boot" note is now
a measured fact, not a reservation.

## Workload A/B — the rate-limited rule ( folded into FAQ `#gain`)

Same day, fixed seeds, `ssdiag` at both ends, power traces: −18.4% (Qwen-Image
edit, 4 refs) and −25.2% (t2v 15 s) where the locked card averaged ~300 W
(rate-limited); **0%** on reference-image-to-video where the locked card already
averaged 362 W at 100% SM (power-limited; unlocking just added watts), plus ~0%
single-pass t2i and quantized-MoE LLM. The rule — **gain = GEMM gain × the
rate-limited fraction** — is in the FAQ since PR #192; the 0% row is its
control.

## A third host shape — discrete TB3 controller (maintainer's own, 2026-11-06)

ASUS G614JV (i9-13650HX, internal 4060 disabled via the hybrid-graphics mode to
avoid the driver conflict, driver 610.74), discrete TB controller **JHL8540** →
XYT-BX dock (**JHL6540**, TB3, ATX forced power), EVGA triple-fan **3090
(crossflashed VBIOS)** — the first GA102 eGPU datapoint and the first
**discrete**-controller host:

- UEFI: external 3090 recognized and unlocked normally.
- Windows: card picked up directly, **unlock retained, gate open, SS green from
  the very first boot** — no suppression recorded on this machine, and a
  discrete TB3 controller usually runs the legacy Thunderbolt stack rather than
  the Microsoft USB4 CM (whether `Usb4HostRouter`/`Usb4DeviceRouter` even exist
  here is unrecorded; a `ssdiag ≥ 0.6.13` `USB4:` block would settle it).
- **Unplug/replug of the TB3 cable (dock kept powered) → unlock lost, gate
  relocked.** So the reset also rides tunnel *re-establishment*, not only the
  initial takeover. Contrast the TB4 machine's replug cell: there (suppressed)
  nobody re-establishes the tunnel and the card just vanishes; here
  (presumably stock stack) the tunnel comes back and the card returns locked.
- Photos: `g614jv-tb3-bench-unlocked.jpg` (ssdiag `VERDICT … UNLOCKED (override
  values live)`, Windows graphics settings listing the 3090 as GPU 0, iGPU as
  GPU 1) and `g614jv-tb3-ssdiag-probe.jpg` (the extended probe surface —
  fuse/TPC banks reading `[PRIV_VIOLATION]` is the expected shape for
  PLM-gated banks this tool never opens).
- Open on this machine: one canary read after a replug (`cleared` = domain
  reset, `intact` = narrow write), and the `USB4:` block.

## Power gating (AD102, locked clocks)

The 2x is real but power-gated, and shows it cleanly: with `nvidia-smi -lgc`,
the unlocked gain over locked falls monotonically with clock - +55% at 1005 MHz
(60 W -> 160 W), +23% at 1800 MHz, +18% at 2100 MHz, +15% at free boost (450 W
ceiling). Cheaper formats move the wall: FP8 (`torch._scaled_mm`) 532.1 TF/s =
~160% of the 4090's official FP8 figure, INT8 593.2 TOP/s = ~180% of spec.
Everyday inference on the host is not tensor-bound at all (identical wall clock
at 2100 and 2595 MHz). This belongs with the power-wall section of
`docs/theoretical-uplift.md`.

## Residual unknowns

- Sleep/resume under suppression (undock/replug is now measured — never comes
  back hot; recovery is a chain reboot).
- Which host-router init call issues the reset (a 100 ms canary/FEAT poller
  aligned with Kernel-PnP event IDs would name it).
- Windows has no narrower knob than service disable: the `Usb4HostRouter` node's
  `Device Parameters` holds only WDF/DMA/interrupt keys.
- Linux driver-loaded end to end (bind `nvidia` with the manager loaded and
  `host_reset=0`): not yet run; would be the first "unlocked and usable" Linux
  state on this host.
- The discrete-TB3 host: whether the Usb4 CM services exist there at all, and a
  canary read after a cable replug (domain reset vs narrow write).
- Driver-update re-arm of `Start=4` (PnP reinstall / `pnputil /scan-devices`):
  untested, labelled as such.

