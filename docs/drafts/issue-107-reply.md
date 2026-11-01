@seti9585 Thank you for the exceptional report — the CSM-on/CSM-off ablation, the pinned hashes, and especially your own working native-GOP implementation. This is exactly the evidence the reconnect hang class needed.

**Where we landed (implemented on branch `issue107-csm-compat`, not yet in a release):**

1. **Your reconnect blind spot is fixed first**: the `DIAG *-OPENERS` log lines now carry the driving image's base+size, so a stuck `ConnectController` log shows which display driver (BiosVideo vs vendor GOP) was being re-selected.
2. **Native-GOP recovery path (`\tx-native-gop.txt` marker, opt-in)** — modeled on your approach, implemented independently in Rust: take the card's own PciBus-shadowed option ROM (or a compatibility module file from the stick), locate the code-type-0x03 EFI image, decompress through the firmware's own `EFI_DECOMPRESS_PROTOCOL`, LoadImage/StartImage it, call its Driver Binding Supported/Start explicitly on the target GPU only, verify the GOP child by device-path ownership, restore the console on the child only (ConnectController on the PCI handle is never called), and restore VGA forwarding through PciIo Attributes with BME kept off. Any failure leaves the console to bootmgfw — never a retry of the hanging path. Decompression being delegated to the firmware protocol was the key simplification: NVIDIA ships every GOP we examined (TU106/AD107) as a CompressionType=1 image, and PciBus itself uses that exact protocol for compressed OPROMs.
3. We evaluated and dropped a Driver####-style preload of the GOP driver — the in-transaction dynamic load covers it, and we prefer zero resident NVRAM state. Your trailing device-ID/checksum normalization work maps to our GOP-replacement backlog; for the runtime-load path we agree the trailing block is unlikely to matter (Supported() checks the PCI device, not the tail) — flagged for confirmation on real hardware.

**Two experiments that would close the class (CSM on, stock v2.1.8 or newer):**

1. `\no-reconnect.txt` — does bootmgfw rebuild the console under CSM? (extends the Code 12 law's boundary to CSM machines)
2. `\tx-reconnect-nonrecursive.txt` — vendor-GOP-only bind behavior under CSM.

And once a build with `\tx-native-gop.txt` ships: single-marker run, full loader log. Your RomImage/GOP extraction hashes (RomImage sha d9bfb451…, GOP sha b69af88e…) will be the known-answer anchors for our extractor on your card.

Licensing note, in the open: your unlock-side sources are GPL-2.0-only (compatible with this project); the GOP-related files carry no license header, so we implemented from the documented behavior only, GPL-2.0-only, no code copied.
