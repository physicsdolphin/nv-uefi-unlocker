# TB4 eGPU RTX 4090 — the OS router takeover is the writer (2026-11-05)

Reporter `Ambolio`, desktop Thunderbolt 4 host, Razer Core X V2 enclosure,
`10de:2684` (AD102). An on-board RTX 5090 owns the console. This is the case
that closes the "tunnel rebuild / PERST# / enclosure power" suspects and makes
the OS-side USB4/Thunderbolt connection manager the **only** surviving writer —
the sixth arm of the canary table in the FAQ, and the only one where the reset
is performed by the OS rather than by a rail, a firmware reset or a driver.

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
- the only lever is to stop the takeover from performing the reset. Linux has
  `thunderbolt.host_reset=0` for exactly that host-router init reset (next
  round, unverified). Windows has no known equivalent; worth one cheap round
  only on the `Usb4HostRouter` node's own `Device Parameters` subkey, and on
  disabling HostRouter and DeviceRouter **separately**.

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

`thunderbolt.host_reset=0` with the `thunderbolt` blacklist removed. Live →
the mechanism is pinned to the driver's reset call and the host gains its first
unlocked-and-usable Linux shape. Lost → the reset is not on the `host_reset`
path and enumeration/tunnel rebuild becomes the target. Also proposed: a 100 ms
poll of `0x1480`/`0x1484` plus the FEAT quad with change-only timestamps,
aligned against Kernel-PnP / `Usb4HostRouter` event IDs, to turn "inside the
takeover window" into "which call".

[#164]: https://github.com/physicsdolphin/nv-uefi-unlocker/issues/164
