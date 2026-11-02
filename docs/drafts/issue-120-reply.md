Thank you for the outstanding report - the opener rows in this log are exactly what the class needed. Your mechanism reading is mostly right; two corrections, and one decisive experiment your log shows was never run.

**What the log actually shows (structure your diagnosis guessed at):**

```
GOP-OPENERS: Handle(0x68293b98) agent=0x69d47a18 controller=0x6828e018 attr=0x8 (BY_CHILD)
GOP-OPENERS: Handle(0x6828e018) agent=0x69d44b98 controller=0x6828e018 attr=0x10 (BY_DRIVER)
```

The pathless GOP (0x6828e018) is not just "a splitter survivor" - it is a **child created by the target GPU's own GOP driver stack** (0x69d47a18 holds it BY_CHILD), and after the recursive PCI disconnect tears the parent stack down, it survives **orphaned** with the ConSplitter console (0x69d44b98) still driving it. PR #13's mitigation covers our connect calls; nothing covers bootmgfw rebinding over the orphan. Your "one step later than the current fix covers" is exactly right.

**Correction 1 - the last log line is not a fault location.** `start_loader` flushes and closes the log *before* `LoadImage` by design (the log must survive the handoff). A fully successful run ends at the same `CHAIN: LoadImage (FromDevicePath)` line. So the log cannot distinguish "reset inside bootmgfw's console rebind" from "reset further inside StartImage" - your conclusion is a reasonable inference (no Windows logo + instant power-cycle puts it early), not a log-proven fact.

**Correction 2 - `no-reconnect.txt` removed the correct mitigation on this board.** v2.1.9-rc1 inherits PR #13's automatic path: when a pathless GOP survives the detach, the loader reconnects **non-recursively, vendor GOP only** (`RECONNECT: pathless GOP survived the detach - vendor GOP only (non-recursive)`) - verified end-to-end on the same board generation (ROG MAXIMUS Z790 APEX + 2803, AD106). That rebind creates a *fresh* vendor GOP before bootmgfw ever runs. `no-reconnect.txt` skips even that, handing bootmgfw an orphaned console child - the exact structure that resets your firmware. Your attached log never shows a default-mode run.

**Requested runs, in this order:**

1. **Default: no markers at all, stock v2.1.9-rc1.** Expected log line: `RECONNECT: pathless GOP survived the detach - vendor GOP only (non-recursive)`. This is the PR#13-proven shape for your board family and the single most important missing data point. No new build needed.
2. If (1) still resets: `	x-reconnect-nonrecursive.txt` on stock rc1 (explicit form of the same path - rules out a stale-detection issue).
3. A diagnostic build adding `	x-orphan-console-teardown.txt` (branch `issue120-orphan-teardown`, on the main lineage): after the detach it disconnects the console drivers from surviving pathless GOPs, so bootmgfw faces the zero-console shape where its own re-establishment is proven (Z170-A control). Full log please - the opener rows before/after the teardown are the point. A binary will be attached to this issue.

Your uninstall-GOP proposal: agreed in direction, but we implement it as DisconnectController of the console driver from the orphan (standard path, no protocol-database surgery) behind the diagnostic marker - same philosophy as your ownership-checked binding isolation.

Once the run (1)/(3) logs land, the board goes into the compatibility matrix either way: Z790+GA106 pathless-orphan shape.
