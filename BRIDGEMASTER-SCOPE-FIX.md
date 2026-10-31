# BridgeMaster guard lifetime bug â fix

`BridgeMaster`'s `Drop` runs too early, so the bridge above the GPU loses bus master
again *before* the booter fires. The comment above the code already claims the guard
"lives until `run_transaction` returns"; the code did not do that.

## The bug

`loader/src/tx.rs`, in `run_transaction`:

```rust
crate::pci::log_dma_path("before-bridge-bme", gpu.pci);
if probe_file("\\tx-no-bridge-master.txt") {
    info!("TX: \\tx-no-bridge-master.txt present - BridgeMaster::enable skipped (guarded-overlay parity)");
} else {
    let _bridge_master = crate::pci::BridgeMaster::enable(gpu.pci);   // <-- bound inside the block
}
crate::pci::log_dma_path("after-bridge-bme", gpu.pci);
```

`let _bridge_master` is scoped to the `else` block, so it is dropped at the closing
brace â which restores the bridge `COMMAND` register and clears bus master again
immediately. The booter then runs with BME clear and fails.

## Reproduction

Any board whose upstream bridge starts with BME clear. On this machine
(ASUS PRIME B760M-F D4, Intel B760, AMI BIOS) the Intel Root Port `8086:a70d`
starts at `command=0x0003` once VT-d is disabled in firmware â `0x0003` has
IO+MEM decode but bit 2 (bus master) clear.

Log on that boot:

```
BRIDGE-BME: Some((0, 0, 1, 0)) command 0x0003 -> 0x0007
BRIDGE-BME: restored Some((0, 0, 1, 0)) command 0x0007 -> 0x0003
DMA-PATH[after-bridge-bme]: bridge 8086:a70d command=0x0003     <- already restored
BOOTER forensic: CPUCTL=0x00000010 MAILBOX0=0x00000089
POST-BOOT: SEC2 MAILBOX0=0x00000089 WPR2_HI=0x1ffae00
HANDOFF-OUT: ... plm_open=false ss0_unlock=false ss1_unlock=false
```

`0x89` is the same "upstream bridge has no bus master" failure documented in
`docs/evidence/ad106-4060ti-desktop/report.md` (PR #5).

Note the ordering: the restore line appears *before* `after-bridge-bme`, i.e. the
guard was already gone while the transaction was still running.

## The fix

Hoist the binding out of the branch so it lives to the end of `run_transaction`:

```rust
crate::pci::log_dma_path("before-bridge-bme", gpu.pci);
let _bridge_master;
if probe_file("\\tx-no-bridge-master.txt") {
    info!("TX: \\tx-no-bridge-master.txt present - BridgeMaster::enable skipped (guarded-overlay parity)");
    _bridge_master = None;
} else {
    _bridge_master = Some(crate::pci::BridgeMaster::enable(gpu.pci));
}
crate::pci::log_dma_path("after-bridge-bme", gpu.pci);
```

The `Option` keeps the marker-file path working without an extra `if` around the
whole transaction.

## Result after the fix

Same machine, same BIOS, same driver, same stick â nothing else changed:

```
BRIDGE-BME: Some((0, 0, 1, 0)) command 0x0003 -> 0x0007
DMA-PATH[after-bridge-bme]: bridge 8086:a70d command=0x0007     <- stays on
BOOTER forensic: CPUCTL=0x00000010 MAILBOX0=0x67b00001
POST-BOOT: SEC2 MAILBOX0=0x67b00001 WPR2_HI=0x1ffae00
SELECTOR RESULT: SS0=0x88888888 SS1=0x8
FINAL: transaction complete, all DMA released
BRIDGE-BME: restored Some((0, 0, 1, 0)) command 0x0007 -> 0x0003   <- restored at the end, as intended
HANDOFF-OUT: ss0=0x88888888 ss1=0x8 readout1=0x00000000 feat_plm=0xffffffff ... plm_open=true
```

Restore now happens after `FINAL`, which is what the original comment described.

Windows side (`ssdiag v0.4.8 --select 2803`) on that boot:

```
ampere/ada 0x823xxx  PLM       = 0xffffffff (OPEN)
ampere/ada 0x823xxx  SS0       = 0x88888888 [0* 0* 0* 0* 0* 0* 0* 0*]
ampere/ada 0x823xxx  SS1       = 0x00000008 [0*]
ampere/ada 0x823xxx  READOUT_1 = 0x00000000 [full speed]
VERDICT [ada]: UNLOCKED (override values live)
```

cuBLAS 4096Â³, median of 3 trials Ã 30 samples:

| Mode | Stock | Unlocked | Gain |
|---|---:|---:|---:|
| FP16 â FP32 | 45.59 | **77.36** | **+69.7%** |
| BF16 â FP32 | 45.64 | **77.74** | **+70.3%** |
| TF32 | 20.06 | **41.91** | **+108.9%** |
| FP32 pedantic (control) | 12.40 | 12.85 | noise |

## Why this stayed hidden

On boards where firmware already leaves BME set on the upstream bridge,
`BridgeMaster::enable` is a no-op, so the premature `Drop` restores nothing and
the bug is invisible. Every desktop success so far (PR #5, #33, #51, #56) was on
such a board. It only appears once something makes the bridge start with BME clear
â here, disabling VT-d.

So the fix is a no-op for existing verified configurations and necessary for the
BME-clear ones.

## Scope

One file, one block: `loader/src/tx.rs`. No behaviour change when the bridge
already has BME; no change to register sequencing, assets, or verdict logic.
