# nv-uefi-unlocker v2.1.11: Windows handoff failure

Hardware: Gigabyte B550M AORUS ELITE; NVIDIA RTX 2060 (10de:1f08), VBIOS 90.06.30.00.3f; approximately 16 GiB RAM.
Windows driver: 621.01; running GSP firmware: 620.67.01. EnableGpuFirmware=1.
EFI SHA-256: 70e48db88d20e9027a44b311336385701ca59d50f55fd84585e7fd751d360646.

Observed error reported by user: Windows Recovery 0xc000009a.

- Stock v2.1.11: transaction completes, POST-GOP selectors remain SS0=0x88888888 and SS1=0x8. Firmware resolver correctly selects Windows ESP 3bef48d4-9ca0-4e27-9351-d478435bad57. Asset cache freed: 16,827,004 bytes. Largest low-memory block: 2497 MiB, 1013 descriptors. Preloaded bootmgfw at 0x10000000. Windows Recovery.
- tx-no-preload.txt: transaction and POST-GOP checks pass. Cache freed: 16,827,004 bytes. Largest low-memory block: 2580 MiB, 1013 descriptors. Correct Windows ESP selected. User reports same 0xc000009a.
- Earlier tx-exit-fw.txt: transaction passes, returns to firmware as expected; user reports black screen resembling a command prompt. Screen identity was not captured.
- Normal Windows boot works. Display device previously reported ConfigManagerErrorCode=0.

Current USB configuration: stock release files plus tx-no-preload.txt. No tx-exit-fw.txt or chain-target.txt.

Unresolved discriminator: boot through the USB and press Escape at the confirmation screen (no unlock transaction), keeping tx-no-preload.txt. Record whether Windows boots and preserve loader.log. Do not repeat an unlock transaction after a fault without a full power-off.

Related upstream report and comments: https://github.com/physicsdolphin/nv-uefi-unlocker/issues/146
Source explanation: https://github.com/physicsdolphin/nv-uefi-unlocker/blob/v2.1.11/docs/handoff-lowmem.en.md
No issue or message has been submitted.

## Escape control confirmed
The latest USB run logged ESCAPE / ABORT / zero GPU writes. User reports Windows booted normally. With tx-no-preload still present: largest block 2580 MiB, 998 descriptors, 93 MiB resident. Real transaction failed at the same largest block, 1013 descriptors, 92 MiB resident. This rules out a simple largest-block threshold as the sole explanation; transaction side effects and descriptor shape remain unresolved. Driver 621.01 unchanged.

## Driver update did not resolve handoff
Driver now 621.14, GSP firmware 620.70.03, EnableGpuFirmware=1, display device error code 0. User reports same failure. Latest unlock attempt retains selectors after native GOP rebuild; cache freed (16,827,004 bytes), largest block 2580 MiB, 1013 descriptors. Same map summary as failing run on 621.01. Saved as loader-driver-62114.log.

## Skip native GOP control
Turing-specific tu10x-no-reconnect.txt was applied with tx-no-preload.txt. User observed a frozen patching screen. Log continued to about 33 seconds and reached the correct Windows loader (CHAIN: LoadImage FromDevicePath), so no stall at the visible patching stage is established. Largest block 2580 MiB, 1014 descriptors. Saved loader-skip-gop-attempt.log. Removed tu10x-no-reconnect.txt afterward to restore the display-recovery default.
