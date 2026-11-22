# V6 Descriptor Reverse Engineering — GB205 Blackwell

- **Date**: 2026-11-22
- **Target**: NVIDIA GeForce RTX 5070 Ti Laptop GPU (PCI `10de:2f18`, SUBSYS `1558:6586`)
- **VBIOS**: 98.05.31.00.44
- **Die**: GB205 (see specimen provenance in §1).
- **Sources**: Full static ROM export and an earlier read-only ssdiag v0.8.5 capture.
- **Purpose**: Test the original four-descriptor observations against all six nonempty entries in this ROM.

---

## 1. Specimens

The archived capture identifies PCI 10de:2f18, subsystem 1558:6586, VBIOS 98.05.31.00.44. PMC_BOOT_0 is 0x1b5000a1. Its upper 12 bits give the GB205 chipset ID 0x1b5. ssdiag reports the broader family blackwell-gb2 (reproduced identically on driver 621.01 in [gb205-bar0-prom-metadata.txt](gb205-bar0-prom-metadata.txt)).

The full ROM is 1,961,983 bytes, SHA-256 `06f94e60c400414cade6f28ebd8c4c2536fd66b5ef22ad094d9f41bea0e2813d`. Its checksummed BIT directory is at 0x361F0. The containing legacy image starts at 0x35400; the following EFI image contributes 0x18000 bytes to the extended-pointer translation. The six descriptors below account for all nonempty entries of the 35-slot ucode table. Four were visible in the original 1 MiB capture. The other two are readable in the full file. These observations cover one VBIOS revision.

| APP | Target | File offset | Version | Header (bytes) | Stored size (bytes) | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| 0x07 | 0x6 | 0xADCD4 | 6 | 80 | 132,608 | 0x1D |
| 0x19 | 0x1 | 0xCE324 | 6 | 80 | 134,912 | 0x5 |
| 0x15 | 0x1 | 0xEF364 | 6 | 80 | 21,760 | 0x5 |
| 0x23 | 0xE | 0xF48F0 | 6 | 80 | 159,744 | 0x1F |
| 0x18 | 0x1 | 0x11BF04 | 6 | 80 | 49,920 | 0x1F |
| 0x24 | 0xF | 0x128904 | 6 | 80 | 175,104 | 0xD |

---

## 2. V6 Field Map (80 Bytes)

Offsets below are hexadecimal and relative to each descriptor. Values through +33 are decoded little-endian scalars. Section 2.4 summarizes the remaining 28 bytes without reproducing them.

### 2.1 Header word (+00)

All six headers have version 6 and size 80. Bits 0 and 2 are set in all six flags. Bit 1 is set on APP 0x18 and 0x23. Bits 3 and 4 are both set on 0x07, 0x18 and 0x23; only bit 3 of this pair is set on 0x24. Neither bit is set on 0x19 or 0x15. The sample alone does not establish an algorithm or an exhaustive class definition.

### 2.2 Structural Fields (+04 to +27, 36B)

| Offset | Size | V2 equivalent | V3 equivalent | APP 0x07 | APP 0x19 | APP 0x15 | APP 0x23 | APP 0x18 | APP 0x24 | V6 observation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| +04 | 4B | stored_size | stored_size | 0x20600 | 0x20F00 | 0x5500 | 0x27000 | 0xC300 | 0x2AC00 | Declared stored size in bytes |
| +08 | 4B | pkc_off(V1) | pkcDataOff | 0x0 | 0x8800 | 0x8800 | 0x0 | 0x0 | 0x15800 | Unconfirmed |
| +0C | 4B | virtualEntry | interfaceOff | 0x1DA00 | 0x1BA00 | 0x4C00 | 0x1D000 | 0x8800 | 0x26C00 | First size term in §2.2.1 |
| +10 | 4B | interfaceOff | imemPhys | 0x0 | 0x3800 | 0x3800 | 0x0 | 0x0 | 0x1400 | Unconfirmed |
| +14 | 4B | imemPhys | imemLoad | 0x1100 | 0x5500 | 0x900 | 0x8800 | 0x1200 | 0x4000 | Second size term in §2.2.1 |
| +18 | 4B | imemLoad | imemVA | 0x200 | 0xC0001 | 0x30001 | 0x0 | 0x40101 | 0x301 | High half matches the manifest row count on 0x19 and 0x15; scope on other APPs is open |
| +1C | 4B | imemVA | dmemPhys | 0x0 | 0x14 | 0x14 | 0x0 | 0x1010014 | 0x1030000 | Unconfirmed |
| +20 | 4B | imemSecBase | dmemLoad | 0x1100 | 0x0 | 0x0 | 0xE00 | 0x1F00 | 0x0 | Third size term where nonzero |
| +24 | 4B | imemSecSize | engineId+sig | 0x0 | 0x0 | 0x0 | 0x0 | 0x0 | 0x0 | Observed scalar; purpose unconfirmed |

### 2.2.1 Stored-size accounting across six descriptors

| APP | +0C + +14 + +20 | stored_size | Residual | +18 high half | Class A outer word after header |
| --- | --- | --- | --- | --- | --- |
| 0x07 | 0x1DA00 + 0x1100 + 0x1100 | 0x20600 | 0xA00 | 0 | differs |
| 0x19 | 0x1BA00 + 0x5500 + 0x0 | 0x20F00 | 0x0 | 12 | matches |
| 0x15 | 0x4C00 + 0x900 + 0x0 | 0x5500 | 0x0 | 3 | matches |
| 0x23 | 0x1D000 + 0x8800 + 0xE00 | 0x27000 | 0xA00 | 0 | differs |
| 0x18 | 0x8800 + 0x1200 + 0x1F00 | 0xC300 | 0xA00 | 4 | differs |
| 0x24 | 0x26C00 + 0x4000 + 0x0 | 0x2AC00 | 0x0 | 0 | differs |

The two original Class A specimens (0x19, 0x15) share a length equality and a recognizable secondary manifest. APP 0x18 shares their target but has the 0xA00 residual, a nonzero third size term, and a different post-descriptor word. Its high-half value of four cannot establish a four-row manifest. APP 0x24 has the two-term equality, a different flag pattern, and no matching Class A outer word. Its residual is zero. The two size patterns divide the sample three to three; a size equality alone does not identify the post-header format.

### 2.3 Constant and Per-Image Fields (+28 to +33, 12B)

| Offset (4-byte scalar) | APP 0x07 | APP 0x19 | APP 0x15 | APP 0x23 | APP 0x18 | APP 0x24 | Observation |
| --- | --- | --- | --- | --- | --- | --- | --- |
| +28 | 0x01012801 | 0x01012801 | 0x01012801 | 0x01012801 | 0x01012801 | 0x01012801 | Same value in 6/6; meaning open |
| +2C | 0x5000000 | 0x4000000 | 0x4000000 | 0x4F4F0000 | 0x4000000 | 0x50534600 | Varies by APP; meaning open |
| +30 | 0x11000000 | 0x12000000 | 0x11000000 | 0x42554842 | 0x2000000 | 0x4E55525F | Printable four-character tags on 0x23 and 0x24 (§4) |

### 2.4 Opaque tail (+34 to +4F, 28 bytes)

All six tails differ from one another. Empirical Shannon entropy over each individual 28-byte sample is shown in bits per byte. Its upper bound for this sample size is log2(28), about 4.807. These figures and the distinctness check do not identify the field's function.

| APP | 0x07 | 0x19 | 0x15 | 0x23 | 0x18 | 0x24 |
| --- | --- | --- | --- | --- | --- | --- |
| Sample entropy | 4.664 | 4.807 | 4.522 | 4.664 | 4.495 | 4.664 |

---

## 3. Class patterns in the original four specimens

### 3.1 Class A — Target 0x01 (Apps 0x19, 0x15)

- Minimal vDesc flags (0x05)
- +08 and +10 are nonzero. Their meanings are unknown.
- +1C equals 0x14 in both Class A samples. Its meaning is unknown.
- The high half of +18 agrees with the manifest row count in these two specimens.
- **Plaintext secondary header** in image data

The secondary headers in these two samples have the same first two decoded words. Other APPs are compared separately in §2.2.1.

Speculative decode of secondary header:
- +0: u16 = 0x0001 (format version or magic low word)
- +2: u16 = 0x0035 (53 — semantics unknown; NOT the header byte count or entry count)
- +4: u32 = 0x000047a7 (18343, constant across both specimens)
- +8: u32 = varies (0x189e0 vs 0x58c0 — correlated with image content, not equal to stored_size)
- +C: u32 = 0x000004bd (1213, constant across both specimens)

**Header length**: The total plaintext header = 0x14 + N × 0x14 bytes, where N = number of engine entries. The entry count N is NOT stored in the secondary header itself — it is derived from V6 descriptor field +18, whose high u16 gives the total row count (including the 20-byte outer header as row 0). See §6.1.

### 3.2 Class B — Targets 0x06, 0x0E (Apps 0x07, 0x23)

- Extended vDesc flags (0x1d, 0x1f — bits 3,4 set)
- +08 and +10 are zero. Their meanings are unknown.
- +1C is zero. Its meaning is unknown.
- Segment C size (+20) is nonzero.
- The Class A secondary-header marker was absent at the first word after both descriptors.

The sampled prefixes lack the Class A header. They do not establish a specific encryption algorithm.

---

## 4. Printable tags at +30

Two of the six scalars at +30 consist of four printable ASCII characters when read in file order. APP 0x23 decodes to `BHUB` and APP 0x24 decodes to `_RUN`. The other four lack a four-character printable tag at that offset. The spelling alone does not identify an engine or establish a class.

---

## 5. V6 vs V2 vs V3 Layout Comparison

```
Offset    V2 (60B, Turing)       V3 (44B, GA10x+)      V6 (80B, Blackwell)
------    ------------------     ------------------     ------------------
+00       vDesc header           vDesc header           vDesc header (+ new flag bits 3,4)
+04       stored_size            stored_size            stored_size
+08       pkc_off (V1)           pkcDataOff             unconfirmed
+0C       virtualEntry           interfaceOff           segment A size
+10       interfaceOff           imemPhys               unconfirmed
+14       imemPhys               imemLoad               segment B size
+18       imemLoad               imemVA                 row count (hi16) on 0x19/0x15; other uses open
+1C       imemVA                 dmemPhys               unconfirmed
+20       imemSecBase            dmemLoad               segment C size
+24       imemSecSize            engineId+sigCount      reserved (zero)
+28       —                      sigVersions            CONSTANT 0x01012801 (semantics unconfirmed)
+2C       —                      beyond V3              per-image byte; APP 0x23 anomalous ("OO")
+30       —                      —                      per-image word; two printable tags (§4)
+34..+3B  imemSecBase..dmemOff   —                      28-byte opaque tail
+3C..+4F  beyond V2              —                      (purpose unconfirmed)
```

V6 fundamentally reorganizes the descriptor: image segment sizes occupy +0C/+14/+20 (§2.2.1 conservation law), and the +28..+4F region spans 40 bytes — a constant dword at +28 followed by per-image fields and a 28-byte opaque tail. This replaces V3's trailing RSA3K signature area (which followed the 44-byte descriptor as variable-length data at pkcDataOff).

---

## 6. Class A Secondary Header — Per-Engine Sub-Image Manifest

### 6.1 Structure Overview

Class A images (apps 0x19, 0x15) contain a plaintext secondary header immediately after the 80-byte V6 descriptor. This header is a **per-entry manifest** whose entries appear to describe firmware segments, though the exact field semantics (source vs destination, offset vs address) remain unconfirmed.

**Total header size** = 0x14 + N × 0x14, where N = number of entries.

**Entry count in the two recognized manifests.** For APP 0x19 and APP 0x15, multiplying the high half of +18 by 20 reproduces the observed manifest length. Its meaning on APP 0x18 remains open.
- APP 0x19: desc+18 = 0x000c0001 → high u16 = 12 → 12 × 0x14 = 0xF0 = 240 bytes ✓
- APP 0x15: desc+18 = 0x00030001 → high u16 = 3 → 3 × 0x14 = 0x3C = 60 bytes ✓
- Class B (0x07): desc+18 = 0x00000200 → high u16 = 0 → no matching Class A prefix ✓
- Class B (0x23): desc+18 = 0x00000000 → high u16 = 0 → no manifest ✓

The low u16 of field +18 (0x0001 for Class A, 0x0200/0x0000 for Class B) may encode a manifest type or format selector.

**Layout:**
```
+000  u32   header_word = 0x00350001 (constant across both specimens)
+004  u32   constant_a  = 0x000047a7 (constant across both specimens)
+008  u32   variable    (varies per image: 0x189e0 / 0x58c0)
+00C  u32   constant_b  = 0x000004bd (identical in both)
+010  u32   constant_c  = 0x00001100 (identical in both)
+014  [N × 20B]  entry table (N varies per image)
...   ciphertext begins immediately after last entry
```

Each entry is 20 bytes (5 dwords). Field roles are determined by the arithmetic closure chains in §6.5:
```
+0   u16   type (0x0001 = standard, 0x0002 = alternate)
     u16   entry_id (per-entry identifier, unique within each manifest)
+4   u32   field_A — start in S (type 1) or in T (type 2)
+8   u32   field_B — start position in space T, type 1; size in space T, type 2 (§6.5.2)
+C   u32   field_C — extent in space S (§6.5.1); zero for type 2
+10  u32   field_D — extent in space T (§6.5.2); zero for type 2
```

Type 1 has measured start/extent pairs in S and T. Type 2 has a start/extent pair in T only. The physical backing of T remains unknown (§6.9.2).
- **field_A** can exceed the parent APP's stored_size (e.g., entry_id 0xd2 in APP 0x15: field_A=0x1936d vs stored_size=0x5500). Space S spans both APPs (§6.5.1 cross-image chain).
- **field_C and field_D** are typically close in value (Δ = 0, +1, or +4); both are zero for type 2 entries.
- **Cross-image identity**: entry_id 0xd2 appears in both APPs with identical field_A, field_C, and field_D values, but different field_B values (§6.4).

### 6.2 APP 0x19 Manifest (11 Entries, 240 Bytes Plaintext)

| # | Entry ID | Type | field_A | field_B | field_C | field_D | C↔D Δ |
|---|----------|------|---------|---------|---------|---------|-------|
| 0 | 0x89 | 1 | 0x55279 | 0x8d00 | 0xad74 | 0xad74 | 0 |
| 1 | 0x12 | 1 | 0x3ec78 | 0x19ae0 | 0x528 | 0x52c | +4 |
| 2 | 0x14 | 1 | 0x26787 | 0x1a010 | 0x1cf1 | 0x1cf5 | +4 |
| 3 | 0x02 | **2** | 0x16570 | 0x2470 | 0 | 0 | — |
| 4 | 0x03 | **2** | 0x16500 | 0x70 | 0 | 0 | — |
| 5 | 0x81 | 1 | 0x4327c | 0x1be30 | 0x3c | 0x40 | +4 |
| 6 | 0x82 | 1 | 0x432b8 | 0x1bd10 | 0x10d | 0x111 | +4 |
| 7 | 0x70 | 1 | 0x1f554 | 0x140a0 | 0x184 | 0x184 | 0 |
| 8 | 0x71 | 1 | 0x1f6d8 | 0x13e58 | 0x244 | 0x244 | 0 |
| 9 | 0x72 | 1 | 0x1f91c | 0x14228 | 0x887 | 0x888 | +1 |
| 10 | 0xd2 | 1 | 0x1936d | 0x158e0 | 0xc0f | 0xc13 | +4 |

Ciphertext begins at header offset +0xF0 (240 bytes from image start).

Note: field_A values, when sorted ascending (0x16500...0x55279), appear to form a monotonically increasing sequence, suggestive of offsets into some address space. However, the reference space is NOT the individual APP's encrypted payload — several field_A values exceed APP 0x19's stored_size (0x20f00).

### 6.3 APP 0x15 Manifest (2 Entries, 60 Bytes Plaintext)

| # | Entry ID | Type | field_A | field_B | field_C | field_D | C↔D Δ |
|---|----------|------|---------|---------|---------|---------|-------|
| 0 | 0x8a | 1 | 0x5ffed | 0x4100 | 0x790 | 0x790 | 0 |
| 1 | **0xd2** | 1 | **0x1936d** | 0x69c0 | **0xc0f** | **0xc13** | +4 |

Ciphertext begins at header offset +0x3C (60 bytes from image start).

Note: Both field_A values exceed APP 0x15's stored_size (0x5500): entry 0x8a has field_A=0x5ffed, entry 0xd2 has field_A=0x1936d. This confirms field_A cannot be an intra-image offset.

### 6.4 Cross-Image Entry Correlation

**Entry ID 0xd2** appears in both manifests with partially identical field values:

| Field | APP 0x19 (entry 10) | APP 0x15 (entry 1) | Match? |
|-------|--------------------|--------------------|--------|
| field_A | 0x1936d | 0x1936d | **YES** |
| field_B | 0x158e0 | 0x69c0 | No |
| field_C | 0xc0f | 0xc0f | **YES** |
| field_D | 0xc13 | 0xc13 | **YES** |

This proves entry_id 0xd2 is a **shared entry** that appears in both Class A manifests with three fields (A, C, D) held constant across images. The only varying field is field_B. This cross-image identity is structural evidence that entry IDs are globally meaningful identifiers (not per-image local indices), and that field_A/C/D encode entry-intrinsic properties while field_B encodes something image-specific.

### 6.5 Arithmetic Closure Chains

Three sets of zero-residual arithmetic relationships lock down the field roles far more tightly than individual value inspection:

#### 6.5.1 Source-Space Contiguity: field_A + field_C = next entry's field_A

Within sequential entry ID groups, each entry's field_A + field_C exactly equals the next entry's field_A:

| From → To | field_A(from) + field_C(from) | = field_A(to) | Residual |
|-----------|-------------------------------|---------------|----------|
| 0x81 → 0x82 | 0x4327c + 0x3c | = 0x432b8 | **0** |
| 0x70 → 0x71 | 0x1f554 + 0x184 | = 0x1f6d8 | **0** |
| 0x71 → 0x72 | 0x1f6d8 + 0x244 | = 0x1f91c | **0** |
| 0x89 → 0x8a *(cross-image)* | 0x55279 + 0xad74 | = 0x5ffed | **0** |

This proves field_A is a start position and field_C is an extent (size) in a shared address space ("space S"). Entries within each group tile space S contiguously. The 0x89→0x8a chain spanning APP 0x19 and APP 0x15 proves both images share the same space S.

#### 6.5.2 Destination-Space Contiguity: field_B + align(field_D) ≈ next entry's field_B

In a different ordering, each entry's field_B + field_D (with small upward alignment) equals the next entry's field_B:

| From → To | field_B(from) + field_D(from) | raw sum | = field_B(to) | Align |
|-----------|-------------------------------|---------|---------------|-------|
| 0x71 → 0x70 | 0x13e58 + 0x244 | 0x1409c | 0x140a0 | +4 |
| 0x70 → 0x72 | 0x140a0 + 0x184 | 0x14224 | 0x14228 | +4 |
| 0x12 → 0x14 | 0x19ae0 + 0x52c | 0x1a00c | 0x1a010 | +4 |
| 0x14 → 0x82 | 0x1a010 + 0x1cf5 | 0x1bd05 | 0x1bd10 | +11 |
| 0x82 → 0x81 | 0x1bd10 + 0x111 | 0x1be21 | 0x1be30 | +15 |
| 0xd2 → 0x03 | 0x158e0 + 0xc13 | 0x164f3 | 0x16500 | +13 |
| 0x03 → 0x02 | 0x16500 + 0x70 | 0x16570 | 0x16570 | **0** |

This proves field_B is a start position and field_D is an extent in a second address space ("space T"), with small alignment padding between entries. Note the ordering in space T differs from space S (e.g., 0x71 precedes 0x70 in T but follows it in S).

For type 2 entries (0x02, 0x03 — which have field_C=field_D=0), field_A and field_B take the roles of start and size *within space T*, chaining with the type 1 field_B allocations.

#### 6.5.3 Outer Header Anchors: +008 and +010 Tie to Entry Boundaries

The secondary header's variable field (+008) and constant (+010 = 0x1100) anchor precisely to entry boundaries in space T:

**APP 0x19:**
- Entry 0x02 terminal: field_A(0x02) + field_B(0x02) = 0x16570 + 0x2470 = **0x189e0** = outer+008 (exact)
- outer+008 + outer+010: 0x189e0 + 0x1100 = **0x19ae0** = field_B of entry 0x12 (exact)

**APP 0x15:**
- outer+008 + outer+010: 0x58c0 + 0x1100 = **0x69c0** = field_B of entry 0xd2 (exact)

This proves outer+008 marks the boundary between type 2 and type 1 allocations in space T, and outer+010 (constant 0x1100) is a fixed gap or reserved region between them.

**Cross-image outer header comparison** (empirically confirmed by S-space probe):

| Field | APP 0x19 | APP 0x15 | Cross-image? | Interpretation |
|-------|----------|----------|-------------|----------------|
| +004 | 0x000047a7 | 0x000047a7 | **constant** | Unknown (newly revealed; possibly capability flags or checksum) |
| +008 | 0x000189e0 | 0x000058c0 | variable | Type 2 terminal in space T (image-specific, §6.5.3) |
| +00c | 0x000004bd | 0x000004bd | **constant** | Unknown (cross-image invariant) |
| +010 | 0x00001100 | 0x00001100 | **constant** | Reserved gap between type 2 and type 1 T-space regions |

Three of four outer header fields are cross-image constants, suggesting they encode VBIOS-global or engine-class properties rather than per-image metadata. Only +008 varies per image (as expected — it tracks the image-specific type 2 allocation boundary in T-space).

#### 6.5.4 Implications for Field Semantics

The arithmetic chains constrain the interpretation to:

| Field | Role | Evidence |
|-------|------|----------|
| field_A | Start in S for type 1; start in T for type 2 | §6.5.1: contiguous tiling, shared across images |
| field_B | Start position in space T (type 1); size in space T (type 2) | §6.5.2: contiguous tiling with alignment; §6.5.3: anchored by outer header |
| field_C | Extent in space S | §6.5.1: field_A + field_C = next field_A |
| field_D | Extent in space T | §6.5.2: field_B + align(field_D) = next field_B |

Type 1 has measured start/extent pairs in S and T. Type 2 has a start/extent pair in T only. The physical backing of T remains unknown (§6.9.2).

### 6.6 Entry ID Namespace Analysis

**Excluded namespace: NV2080_ENGINE_TYPE.** The RM engine type enum (from `cl2080_notification.h` in open-gpu-kernel-modules) defines engine types 0x00–0x53 (NV2080_ENGINE_TYPE_LAST = 0x54). The V6 manifest entry IDs include values 0x70, 0x81, 0x82, 0x89, 0x8a, and 0xd2 — all well above the RM enum's maximum. These entry IDs are therefore not NV2080_ENGINE_TYPE values.

**What is known:** These 12 IDs form a VBIOS-internal namespace distinct from RM's software engine enumeration. Their specific routing semantics (whether they address hardware engines, firmware segments, memory regions, or something else) cannot be determined from the manifest structure alone and require NVIDIA-internal source or open-source driver references to confirm.

**Structural grouping** (observed, not interpreted as hardware identity):

| Group | IDs | Count | Shared Structural Traits |
|-------|-----|-------|--------------------------|
| Low | 0x02, 0x03 | 2 | Type 2, zero field_C/field_D, small field_B values |
| Mid | 0x12, 0x14 | 2 | Type 1, C↔D Δ=+4, moderate field_B values |
| 0x7x | 0x70, 0x71, 0x72 | 3 | Type 1, C↔D Δ=0 or +1, similar field_B values (~0x14000) |
| 0x8x-low | 0x81, 0x82 | 2 | Type 1, C↔D Δ=+4, largest field_B values (~0x1bd00) |
| 0x8x-high | 0x89, 0x8a | 2 | Type 1, C↔D Δ=0, appear in different APPs (0x89→0x19, 0x8a→0x15) |
| Isolated | 0xd2 | 1 | Type 1, C↔D Δ=+4, shared across both APPs (§6.4) |

Sequential IDs within each group suggest a systematic numbering scheme, but the assignment principle (PRI topology, die-level routing, or something else) is unknown.

### 6.7 Entry ID Summary

| Entry ID | Appears In | Type | Distinguishing Observations |
|----------|-----------|------|-----------------------------|
| 0x02 | 0x19 | 2 | Type 2, zero field_C/field_D. field_A contiguous with 0x03 (0x03's field_A + field_B = 0x02's field_A). |
| 0x03 | 0x19 | 2 | Type 2, zero field_C/field_D. Smallest field_B (0x70). |
| 0x12 | 0x19 | 1 | C↔D Δ=+4, field_B=0x19ae0. |
| 0x14 | 0x19 | 1 | C↔D Δ=+4, field_B=0x1a010 (similar to 0x12). |
| 0x70 | 0x19 | 1 | C↔D Δ=0, field_B≈0x14000. Sequential trio with 0x71, 0x72. |
| 0x71 | 0x19 | 1 | C↔D Δ=0, field_B≈0x13e00. |
| 0x72 | 0x19 | 1 | C↔D Δ=+1 (differs from 0x70-71 Δ=0), field_B≈0x14200. |
| 0x81 | 0x19 | 1 | C↔D Δ=+4, largest field_B (0x1be30). Sequential pair with 0x82. |
| 0x82 | 0x19 | 1 | C↔D Δ=+4, field_B=0x1bd10 (near-identical to 0x81). |
| 0x89 | 0x19 only | 1 | C↔D Δ=0, field_B=0x8d00. |
| 0x8a | 0x15 only | 1 | C↔D Δ=0, field_A=0x5ffed (highest of all entries). |
| 0xd2 | Both | 1 | C↔D Δ=+4. Only cross-image entry (§6.4). No sequential neighbors. |

The archived BIT directory lists 20 tokens. After omitting one zero-length token and using PROM coordinates for both sides, none of the 19 top-level targets equals a start among the 10 distinct type 1 entries. The two type 2 entries have no source range to compare. The capture lists directory pointers but does not provide the secondary table contents needed for a complete comparison. A shared address would establish a reference to the same bytes; the referring field would still need a documented meaning to assign a table identity. The separate engine-enum comparison is in §6.6. (See [gb205-bar0-prom-metadata.txt](gb205-bar0-prom-metadata.txt) for the sanitized BAR0 / PROM BIT summary.)

### 6.8 field_C / field_D Δ Pattern

Across the 10 distinct type 1 entries, D − C is zero in four cases, +1 in one case and +4 in five cases. Rounding a length up to the nearest multiple of four can add at most three bytes. It cannot explain the five +4 cases; 0x3C → 0x40 even starts from a multiple of four. For 0x72, 0x887 → 0x888 is compatible with rounding but has no unique explanation. The capture records lengths; it contains no observation of an added tag, length word or checksum. The gaps between consecutive T ranges discussed in §6.5.2 are a separate quantity.

### 6.9 Physical Identity of Spaces S and T

#### 6.9.1 Space S: VBIOS-Relative PROM Offsets (base = adjust)

**Hypothesis**: field_A values are offsets relative to the same `adjust` base (0x4D400) used by the BIT ucode table for descriptor lookups. Under this interpretation, `PROM[adjust + field_A]` is the source data address for each type 1 entry.

**Verification — all S-space addresses fall within the VBIOS data region:**

The PROM layout has a clear boundary: the first firmware descriptor (APP 0x07) begins at PROM+0xADCD4. Everything before that address is VBIOS data (PCI ROM headers, BIT tables, devinit scripts, power tables, display configuration, etc.).

| Type 1 entry | field_A | PROM offset | Signed offset from first descriptor |
|---|---:|---:|---:|
| 0xd2 | 0x1936d | 0x6676D | −0x47567 |
| 0x70 | 0x1f554 | 0x6C954 | −0x41380 |
| 0x71 | 0x1f6d8 | 0x6CAD8 | −0x411FC |
| 0x72 | 0x1f91c | 0x6CD1C | −0x40FB8 |
| 0x14 | 0x26787 | 0x73B87 | −0x3A14D |
| 0x12 | 0x3ec78 | 0x8C078 | −0x21C5C |
| 0x81 | 0x4327c | 0x9067C | −0x1D658 |
| 0x82 | 0x432b8 | 0x906B8 | −0x1D61C |
| 0x89 | 0x55279 | 0xA2679 | −0x0B65B |
| 0x8a | 0x5ffed | 0xAD3ED | −0x008E7 |

Type 2 entries 0x02 and 0x03 have ranges in T only (§6.5.2, §6.8).

The exclusive end is 0xADB7D. The gap to the first observed descriptor is 0xADCD4 − 0xADB7D = 0x157 (343 bytes).

**Why adjust and not image_base**: The BIT ucode table uses `adjust` (= expansion_offset_abs, computed as image_base + expansion_offset_within_image = 0x35400 + 0x18000 = 0x4D400) for all PROM-relative address translation. The manifest entries use the same coordinate system — this is the natural VBIOS offset space.

The listed type 1 ranges precede the first observed descriptor. The capture records bytes from PROM, without tracing subsequent use of those bytes.

**Scope.** The calculated PROM offsets match the archived type 1 sample rows.

**Empirical confirmation (ssdiag v0.8.5, S-space probe patch, admin PROM read):**

Shannon entropy at PROM[adjust+field_A] (256-byte window):
- Entry 0x89 (APP 0x19 primary, fc=0xad74): **2.36 bits/byte** — structured VBIOS data
- Entry 0x8a (APP 0x15 primary, fc=0x790): **0.18 bits/byte** — near-empty (mostly 0x00 fill)

These entropy estimates describe the two 256-byte samples. They do not identify an algorithm or establish the encoding of an entire image.

**S-space data header classification** (first bytes at PROM[adjust+field_A]):

| Prefix (scalar) | Entries | Count | Notes |
|--------|---------|----------------------|-------|
| 0x10 | 0x89, 0x8a, 0x81, 0x82, 0x70, 0x71, 0x72 | 7 | IDs in 0x70–0x8x range |
| 0x30 | 0x12, 0x14, 0xd2 | 3 | IDs outside that range |

For 0x89 and 0x8a, byte 2 matches the entry ID in both samples. Treating bytes 1, 2 and 3 (zero-based) as header size, record size and count gives an exact match to field_C for 5 of the 10 distinct type 1 entries. Among prefix 0x10 entries, 4 of 7 match; among prefix 0x30 entries, 1 of 3 matches. The other five disagree with that particular layout. The prefix could still encode a version for some tables. Its meaning and table family remain unverified; entropy alone identifies neither.

**Cross-image shared entry verification.** Both archived probes for entry 0xd2 returned identical sampled data at PROM+0x6676D.

#### 6.9.2 Space T: Target Destination Address Space

For type 1 entries, field_B gives a start coordinate in T. The physical interpretations are discussed below.

**APP 0x19 ranges in increasing T-coordinate order.** Endpoints are exclusive.

| Region | T Range | Size (bytes) | Contents |
|--------|---------|------|----------|
| desc+08 base | 0x8800 | — | desc+08 value; possible T-space allocation start |
| gap | 0x8800–0x8d00 | 1,280 | Gap from the tentative base to the first entry |
| type 1 low | 0x8d00–0x164f3 | 55,283 | Entries 0x89, 0x71, 0x70, 0x72, 0xd2 |
| alignment gap | 0x164f3–0x16500 | 13 | Between listed ranges |
| type 2 | 0x16500–0x189e0 | 9,440 | Entries 0x03, 0x02 (data-only regions) |
| reserved | 0x189e0–0x19ae0 | 4,352 | outer+010 constant gap |
| type 1 high | 0x19ae0–0x1be70 | 9,104 | Entries 0x12, 0x14, 0x82, 0x81 |
| total span | 0x8800–0x1be70 | 79,472 | From desc+08 to last entry end |

**Boundary clues:**
- desc+08 (0x8800) sits 0x500 below the first type 1 entry (0x89 at 0x8d00). If desc+08 is a T-space base or allocation start, the 0x500 gap could be a header/metadata region.
- outer+010 (0x1100, constant across both Class A images) is a fixed reserved gap between the type 2 terminal and the next type 1 block (§6.5.3).
- Exclusive T endpoint: 0x1be70 = 111.6 KiB. The target engine (PMU, target 0x01) DMEM capacity is unknown (PMU registers are PLM-gated). T-space may address on-die DMEM, VRAM buffer (FRTS/WPR2), or virtual address space — see §6.9.2 capacity cross-check.

**APP 0x15 T-space**: entries 0x8a (T=[0x4100, 0x4890)) and 0xd2 (T=[0x69c0, 0x75d3)). Note that 0xd2 appears in both APP manifests with **different field_B** (0x158e0 in APP 0x19 vs 0x69c0 in APP 0x15), confirming field_B is image-specific (T coordinate varies by image) while field_A/C/D are entry-intrinsic (same PROM source data).

**DMEM capacity cross-check**:

**Target engine identification**: Class A images (APP 0x19/0x15) carry V6 descriptor target 0x01, which in NVIDIA VBIOS convention corresponds to PMU (NV_PPWR, BAR0+0x10a000), not GSP (0x110000) or SEC2 (0x840000). The PMU's on-die DMEM capacity is independent of GSP/SEC2 and has historically been larger on some architectures.

1. **Source code**: Target 0x01 (PMU) DMEM capacity not found in open-gpu-kernel-modules (kernel_gsp_gb202.c, kernel_gsp_gb100.c, kernel_gsp.c, kernel_falcon.c — none contain per-engine DMEM size constants). Published Blackwell headers not reachable via public raw URLs.

2. **Hardware registers** (from the earlier 2026-11-22 driver 616.64 experimental ssdiag probe; standard ssdiag 0.8.8 leaves these six engine-capability registers out of default `CORE_ROWS`):

| Register | Address | Value | Notes |
|----------|---------|-------|-------|
| PMU_CAPS | 0x10a108 | **0xbadf5040** | PRIV_VIOLATION — entire PMU block PLM-gated |
| PMU_HWCFG3 | 0x10a0f0 | **0xbadf5040** | PRIV_VIOLATION |
| GSP_CAPS | 0x110108 | 0x80420100 | Readable (but wrong engine for target 0x01) |
| GSP_HWCFG3 | 0x1100f0 | 0x00000f00 | Readable |
| SEC2_CAPS | 0x840108 | 0x80420100 | Readable (same core config as GSP) |
| SEC2_HWCFG3 | 0x8400f0 | 0x00000f00 | Readable |

Both sampled PMU capacity registers returned PRIV_VIOLATION. GSP and SEC2 readings describe different engines and cannot establish PMU capacity.

3. **Range and capacity.** The observed exclusive endpoint is 114,288 (0x1BE70). With the tentative base of 0x8800, the span is 79,472 bytes. The register reads do not establish physical capacity.
- **H-CAPS-A**: PMU on-die DMEM ≥ 128 KiB (0x20000), the smallest power-of-2 that fits. PMU engines on some prior architectures have had DMEM larger than GSP/SEC2.
- **H-CAPS-B**: T-space addresses are virtual, mapped through the engine's RISC-V standard paging (e.g., Sv39) or NVRISCV MPU to physical DMEM plus external memory, allowing a larger address range than physical DMEM.
- **H-CAPS-C**: T-space destinations are not on-die DMEM at all, but offsets into a VRAM buffer (e.g., the ~1 MiB FRTS region within WPR2). In modern NVIDIA boot flows, VBIOS runtime configuration tables (which S-space confirmed as plaintext VBIOS data, §6.9.1) are commonly DMA-copied to protected VRAM regions rather than squeezed into microcontroller SRAM. Under this interpretation, 111.6 KiB fits trivially.

H-CAPS-C is the preferred interpretation in this note. The observations do not distinguish VRAM, on-die DMEM and a virtual address space.

#### 6.9.3 Adjacent descriptor boundaries in the full file

For each adjacent pair in file order, the exclusive end below is the current descriptor offset plus 80 bytes, its declared stored size, and any independently identified Class A manifest. A positive difference is an interval unaccounted for by this calculation. Its contents need not be padding.

| APP pair | Calculated exclusive end | Next descriptor | Difference (bytes) |
|---|---:|---:|---:|
| 0x07 → 0x19 | 0xCE324 | 0xCE324 | 0 |
| 0x19 → 0x15 (240-byte manifest) | 0xEF364 | 0xEF364 | 0 |
| 0x15 → 0x23 (60-byte manifest) | 0xF48F0 | 0xF48F0 | 0 |
| 0x23 → 0x18 | 0x11B940 | 0x11BF04 | 1,476 (0x5C4) |
| 0x18 → 0x24 (descriptor + stored size only) | 0x128254 | 0x128904 | 1,712 (0x6B0) |

For APP 0x18, 0x11BF04 + 0x50 + 0xC300 = 0x128254. Even tentatively counting four extra 20-byte rows would still leave 0x660 bytes. The observed word following 0x18 differs from the marker in the two known manifests, so this alternative is not used in the table. The two positive intervals each contain a PCI expansion-image header (0x11BA00 and 0x128400 respectively). The final APP, 0x24, has a descriptor-plus-stored-size end of 0x153554 within the file; it has no successor in this directory.

---

## 7. Limits of the evidence

The archived readback establishes sample values and geometric relationships. Runtime parser behavior and authentication order were not observed.

### 7.1 Previously proposed C3 (withdrawn)

C3 is withdrawn. For both Class A samples the observed spacing accounts for the manifest separately from stored_size. The segment sizes in §2.2.1 already account for stored_size. The next visible descriptor provides a sample boundary for the arithmetic in §6.9.3.

### 7.2 Class comparison

In these samples, the semantics of fields +08, +10 and +1C remain unconfirmed. The size relationships for segments A, B and C are listed in §2.2.1. The 0xA00 residual occurs in three entries. Its purpose is unknown.

### 7.3 Destination storage

T denotes a target destination address space. Its backing and capacity remain unverified (§6.9.2).

---

## 8. Open Questions

1. **~~Secondary header full size~~** → RESOLVED: Full manifest captured. APP 0x19 has 240 bytes plaintext (20B header + 11×20B entries), APP 0x15 has 60 bytes (20B header + 2×20B entries).
2. **Row count in the two recognized manifests.** On 0x19 and 0x15 the high half of +18 times 20 gives the observed size including the outer header. The role of the same halfword on 0x18 is open (§2.2.1).
3. **+28 constant.** The same scalar, 0x01012801, appears in all six descriptors. Its meaning beyond this VBIOS remains open.
4. **Printable tags.** The function of the +30 tags on APP 0x23 and 0x24 remains unknown (§4).
5. **Descriptor version and size on APP 0x18 and 0x24 — resolved for this file.** Both are V6 with 80-byte headers. Their physical locations are outside the old 1 MiB capture. Their post-header interpretation is qualified in §2.2.1.
6. **Opaque 28-byte tail.** All six tails differ. Their function remains open (§2.4).
7. **Type 1 source coordinates.** The smallest listed start is PROM+0x6676D. The largest exclusive end is PROM+0xADB7D. Individual ranges and the two sampled entropy estimates are in §6.9.1.
8. **Manifest geometry.** The sampled start/extent relationships are established (§6.5). **Space T backing.** Physical identity remains open (§6.9.2).
9. **Header word 0x0035**: The u16 at secondary header +2 is constant (0x0035 = 53) across both Class A specimens. Its role is unknown — it is neither the entry count (which comes from V6 desc +18) nor the header byte length.
10. **Outer constants +004 and +00C.** With the proposed base and offset/length interpretation, the candidate interval is [0x51BA7, 0x52064), 1,213 bytes. It lies within the nominal 1 MiB window. The archived textual capture does not dump bytes from that interval. The offset/length interpretation remains open. Recognizable bytes alone would not establish that the fields reference them, and irregular-looking bytes would not exclude a valid reference.

---

## 9. Methodology

- Directory and six-descriptor measurements use a local, read-only full ROM export identified in §1. Earlier measurements explicitly attributed to ssdiag use the archived 1 MiB window.
- The full-file comparison reads decoded little-endian scalars and summarizes the opaque tail statistically.
- Class A secondary headers: 256 bytes dumped per image (auto-detected via magic word check on first dword).
- The original four descriptors supplied the hypotheses. APP 0x18 and APP 0x24 test those hypotheses on additional entries from the same VBIOS.
- V2/V3 field positions from `loader/src/tx_fwsec.rs` (the project's own tested implementation).
- Engine ID correlation proved by matching offset/param values for engine 0xd2 across both Class A images.
- S-space physical identity confirmed by ssdiag v0.8.5 S-space probe: Shannon entropy measurement (256-byte window at PROM[adjust+field_A]) and first-byte header classification across all type 1 entries in both Class A images.
- NV2080_ENGINE_TYPE enum sourced from `open-gpu-kernel-modules` HEAD `src/common/sdk/nvidia/inc/class/cl2080_notification.h` (MIT license, 2022-2026). Full enum spans 0x00–0x53 (84 numeric values). Used to exclude RM engine types as the namespace for V6 manifest entry IDs (§6.6).
