# #122 FAT cluster-carve evidence (2026-11-06)

Two log sessions recovered by the #122 reporter from a full-disk image of the boot stick (BPB parse + per-cluster scan). Attributed in the issue as the two deepest of 37 carved sessions; both are v2.1.9 tx and both end on the same line.

| File | SHA-256 | Lines | Ends at |
|---|---|---|---|
| `recovered-session-A-top-v219tx.txt` | `8385656ead9c0b0a6fca628a022aef949e7c63ad74154404c64eb2ce50193d4a` | 156 | `DETACH: PCI disconnect status Ok(()) protocols 2 -> 2` (7607 ms) |
| `recovered-session-B-deepest-v219tx.txt` | `69016269f573f219b6fddd9446bf7907c317308dbca149a1983a3f2e4a2bf6e4` | 161 | same line (26687 ms) |

Source: <https://github.com/physicsdolphin/nv-uefi-unlocker/issues/122#issuecomment-6674373086> (attachments 34456448 / 34456449). Untrusted third-party text, byte-exact as downloaded; no trailing CRLF on the final line.

What they establish, and how they are read, are recorded in [docs/diag-2.1.12-diag3-protocol.md](../../diag-2.1.12-diag3-protocol.md) §6. Two facts to keep in mind when reading them directly:

- the writer is `info!` = file write **first**, console mirror second, so a death inside that pair truncates the file exactly at the line being echoed - a clean cut on a meaningful line is the expected shape, not a coincidence;
- every line here is `Flush`ed (`tx-flush-lines=1`), which on this platform does not commit the dirent but does push the FAT cache to the medium - hence intact text with a zero-byte dirent.
