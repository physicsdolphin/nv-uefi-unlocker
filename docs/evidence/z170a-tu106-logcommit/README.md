# Z170-A/TU106 log-commit ladder bench evidence (2026-11-08)

`loader-log-first-main-run.log` = the FIRST main-loader run with any log
at all (log-commit=0 flush-only, 17,716 bytes): the ladder fix verified
live on the board that 0-byted every diag3-5-era main build.

Key lines for the open GSP-swap question (615 fwimage left in the root):
- `\fwimage_tu10x.bin loaded 22887880/22887880` - the 615.78.08 swap, NOT stock
- `POST-BOOT: FEAT_PLM=0xffffffff SS0=0x11000 SS1=0x1` - PLM opened, selectors NOT written
- `POST-BOOT: tu10x selectors not written ... - the payload's writes did not happen`
- `TX: transaction failed: EIO` - the new selector-based verdict firing

Partial-execution shape (PLM gadget landed, SS gadgets did not; DMEM fill
check shows sanitize intact - the #90 false-negative clause covers
"PLM+selectors both landed", NOT this). Attribution open between:
A. 615 fwimage (badge-theory datapoint: 615 half-opens) - discriminators:
   stock fwimage + this same loader; 560/610 succeeded on the OLD loader.
B. new-loader transaction regression (v2.1.10..main, 153 commits; the
   v2.1.11-diag1..5 tags are ready-made bisect snapshots).

`new-session.log` = the carve that turned out to be the loader BINARY's
own string table (the "BOOT: logger init returned" whole-image single hit
lives there, not in a log session) - negative evidence for "the blind run
wrote nothing", consistent with the dirent holding first_cluster=0.
`fatrecover.py` / `carve-log.py` = the FAT32 dirent-chain walk and the
raw pattern carve used against a raw volume image (admin imaging script:
image-g.ps1, not archived).
