//! M1 read-only cold-state sampler: BAR0 register table + badf bucketing +
//! PROM identity CRC. Pure reads; the only writes anywhere are log lines.
//!
//! Register table from task brief §3.2 (community cross-verified values;
//! PMC_BOOT_0 independently confirmed against open-gpu-kernel-modules
//! swref dev_pmc.h = 0x00000000).

use alloc::vec;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams};
use uefi::Status;

use crate::pci::{FoundGpu, PciIo};

const BAR0: u8 = 0;

struct Reg(&'static str, u32);

/// Cross-family cold-state table (task brief §3.2).
const TABLE: &[Reg] = &[
    Reg("PMC_BOOT_0 chip-id", 0x0000_0000),
    Reg("FB_SIZE_MIB 0x1183a4", 0x0011_83a4),
    Reg("VGA_WORKSPACE 0x625f04", 0x0062_5f04),
    Reg("GFW_PLM 0x118128", 0x0011_8128),
    Reg("GFW_SCRATCH 0x118234", 0x0011_8234),
    Reg("GSP_CPUCTL 0x110100", 0x0011_0100),
    Reg("PGC6_BSI_SCRATCH15 0x1180fc", 0x0011_80fc),
    Reg("MMU_PLM 0x1fa7c8", 0x001f_a7c8),
    Reg("MMU_LOCK_LO 0x1fa82c", 0x001f_a82c),
    Reg("MMU_LOCK_HI 0x1fa830", 0x001f_a830),
    Reg("WPR2_LO 0x1fa824", 0x001f_a824),
    Reg("WPR2_HI 0x1fa828", 0x001f_a828),
    Reg("SEC2_CPUCTL 0x840100", 0x0084_0100),
    Reg("FUSE_OPT_FEAT_OVR_DISABLE 0x8203f0", 0x0082_03f0),
    Reg("FUSE_VER 0x824140", 0x0082_4140),
    Reg("FUSE_VER_DECODE 0x8241c0", 0x0082_41c0),
    // AD10x-specific (64G BAR1 craft). On TU106 these are expected to
    // bucket as badf; that is itself a data point, not an error.
    Reg("AD1xx CYA 0x88724", 0x0008_8724),
    Reg("AD1xx CFG 0x88dcc", 0x0008_8dcc),
    Reg("AD1xx XVE_PLM 0x88ff4", 0x0008_8ff4),
    Reg("AD1xx BAR1_CAP_MIRROR 0x88bbc", 0x0008_8bbc),
    // Ada four-write chain targets (gpu-unlock-ada); cold values on AD107M
    // are an open measurement (next real-machine run answers them).
    Reg("ADA WPR_PLM 0x1fa7c4", 0x001f_a7c4),
    Reg("ADA WPRCFG_PLM 0x1fa7cc", 0x001f_a7cc),
    Reg("ADA FBPA_PLM 0x9a0148", 0x009a_0148),
    // The REAL Turing FEAT block (cmp50hx lines 294-296; the 0x823xxx sweep
    // measures a different locked block on Turing).
    Reg("TU10x FEAT_PLM 0x409650", 0x0040_9650),
    Reg("TU10x FEAT_SS0 0x409664", 0x0040_9664),
    // Readout of the same block (nlzy turing-unlock, same constant ssdiag
    // carries). Cold-state read doubles as the #10 factory-clamp probe once
    // a payload run opens the gate.
    Reg("TU10x FEAT_READOUT 0x409668", 0x0040_9668),
];

/// FEAT override cluster, dword sweep (task brief: 0x823804-0x823820 focus;
/// cmp170hx: locked PL0 sees live dwords 0x823800-0x82382C + 0x823B00).
const FEAT_CLUSTER: core::ops::Range<u32> = 0x0082_3800..0x0082_3830;
const FEAT_ROW_REMAPPER_PLM: u32 = 0x0082_3b00;

const PROM_WINDOW: u64 = 0x0030_0000; // NV_PROM (VBIOS image) inside BAR0
const PROM_CRC_DWORDS: usize = 0x4000; // 64 KiB identity pass

/// badf poison bucketing (task brief §3.3): all-badf is NOT one bucket.
/// Unknown high-half badf codes (e.g. the 0xbadf5620 first measured on
/// AD107 SEC2_CPUCTL) fall into BADF_VARIANT instead of masquerading as ok.
fn classify(v: u32) -> &'static str {
    if v == !0u32 {
        "NO_DECODE"
    } else if v == 0xBADF_5040 {
        "PRIV_VIOLATION"
    } else if v == 0xBADF_1100 {
        "UNDECODED"
    } else if v & 0xFFFF_FF00 == 0xBADF_2000 {
        "FLOORSWEPT"
    } else if v >> 16 == 0xBADF {
        "BADF_VARIANT"
    } else {
        "ok"
    }
}

pub fn run(gpu: &FoundGpu) {
    info!("PHASE: SAMPLE (M1 read-only cold state)");
    info!(
        "SAMPLE: GPU 10de:{:04x} @ {:04x}:{:02x}:{:02x}.{}",
        gpu.device_id, gpu.seg, gpu.bus, gpu.dev, gpu.fun
    );
    let proto = unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams {
                handle: gpu.handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    };
    let pci = match proto {
        Ok(p) => p,
        Err(e) => {
            info!("SAMPLE: open PciIo failed: {:?}", e.status());
            return;
        }
    };

    for Reg(name, off) in TABLE {
        let v = pci.bar_read_u32(BAR0, *off as u64);
        info!("SAMPLE {name} = {v:#010x} [{}]", classify(v));
    }

    // FEAT cluster sweep: the TU106-existence open question answers here.
    // Dword-stride only: u32 reads at non-4-aligned offsets are rejected by
    // the device (first measured on AD107: 36 misaligned reads -> UNSUPPORTED).
    let mut ok = 0usize;
    let mut badf = 0usize;
    let mut no_decode = 0usize;
    let mut errors = 0usize;
    let mut non_badf_values: alloc::vec::Vec<(u32, u32)> = alloc::vec::Vec::new();
    let mut sweep_offsets: alloc::vec::Vec<u32> =
        (FEAT_CLUSTER.clone().step_by(4)).collect();
    sweep_offsets.push(FEAT_ROW_REMAPPER_PLM);
    for off in sweep_offsets {
        match pci.bar_try_read_u32(BAR0, off as u64) {
            Some(v) => match classify(v) {
                "ok" => {
                    ok += 1;
                    non_badf_values.push((off, v));
                }
                "NO_DECODE" => no_decode += 1,
                _ => badf += 1,
            },
            None => errors += 1,
        }
    }
    let v = pci.bar_read_u32(BAR0, FEAT_ROW_REMAPPER_PLM as u64);
    let _ = v; // 0x823b00 already swept above; headline read kept for the log line below
    info!(
        "SAMPLE FEAT sweep 0x{:06x}-0x{:06x}+0x{:06x} (dword stride): {} ok / {} badf / {} all-ones / {} read-errors",
        FEAT_CLUSTER.start,
        FEAT_CLUSTER.end - 1,
        FEAT_ROW_REMAPPER_PLM,
        ok,
        badf,
        no_decode,
        errors
    );
    for (off, v) in non_badf_values {
        info!("SAMPLE FEAT 0x{off:06x} = {v:#010x} [ok]");
    }
    if ok == 0 && no_decode > 0 {
        // 0xffffffff means the BAR0 window is not decoding at all (MEM off at
        // handoff, or the dGPU is powered off). The claim enables MEM later;
        // an all-ones card that never changes is D3cold (iGPU-only boot).
        info!("SAMPLE: BAR0 not decoding at sample time (all-ones) - MEM enable happens in the claim; a card that stays all-ones after it is powered off (D3cold)");
    } else if ok == 0 {
        info!("SAMPLE FEAT cluster: NOT readable at PL0 on this chip (all badf) - recorded as the answer");
    }
    // FEAT_READOUT_1 headline: 0x00000000 = full speed, non-zero = throttled.
    let readout = pci.bar_read_u32(BAR0, 0x0082_3818);
    info!("SAMPLE FEAT_READOUT_1 = {readout:#010x} (0 = no compute throttle active)");

    // PSTRAPS (envytools hw/io/pstraps.html; NvStrapsReBar SetupNvStraps.c).
    // G80+: BAR1 size = SET0[15:14] + SET1[22:20] summed (0=64MiB, each +1
    // doubles, 10=64GiB); SET1[19:17]=BAR0 size index (16MiB<<idx);
    // SET1[23]=BAR3 size (0=2*BAR0, 1=1*BAR0); bit31=override-enable (0
    // restores ROM straps on reset). Turing ReBAR is real and works through
    // exactly these straps (NvStrapsReBar, verified GTX 16xx / RTX 20xx) -
    // the factory value is the read-only half of the TU106 BAR1 question.
    const BAR1_SIZES: [&str; 11] = [
        "64MiB", "128MiB", "256MiB", "512MiB", "1GiB", "2GiB", "4GiB", "8GiB", "16GiB", "32GiB",
        "64GiB",
    ];
    let straps0 = pci.bar_read_u32(BAR0, 0x0010_1000);
    let straps1 = pci.bar_read_u32(BAR0, 0x0010_100c);
    info!(
        "SAMPLE PSTRAPS SET0 0x101000 = {straps0:#010x} [{}] / SET1 0x10100c = {straps1:#010x} [{}]",
        classify(straps0),
        classify(straps1),
    );
    if classify(straps0) == "ok" && classify(straps1) == "ok" {
        let bar1_idx = ((straps0 >> 14) & 0x3) + ((straps1 >> 20) & 0x7);
        let bar0_idx = (straps1 >> 17) & 0x7;
        info!(
            "SAMPLE PSTRAPS decode: BAR1 index {bar1_idx} = {}  BAR0 index {bar0_idx} (16MiB<<idx)  BAR3={}BAR0  override_en={} (0 = ROM straps win on reset)",
            BAR1_SIZES.get(bar1_idx as usize).copied().unwrap_or("reserved(>10)"),
            if straps1 & 0x0080_0000 != 0 { "1x" } else { "2x" },
            straps0 >> 31,
        );
    }

    // PROM identity: CRC32 of the first 64 KiB of the VBIOS image window.
    let mut buf = vec![0u32; PROM_CRC_DWORDS];
    match pci.bar_read_u32_buf(BAR0, PROM_WINDOW, &mut buf) {
        Status::SUCCESS => {
            let bytes = unsafe {
                core::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4)
            };
            match boot::calculate_crc32(bytes) {
                Ok(crc) => info!(
                    "SAMPLE NV_PROM first {:#x} bytes: crc32 {crc:#010x}, magic {:#010x}{}",
                    buf.len() * 4,
                    buf[0],
                    if buf[0] == !0u32 { " (window all-ones - CRC not an identity)" } else { "" }
                ),
                Err(e) => info!("SAMPLE NV_PROM crc failed: {:?}", e.status()),
            }
        }
        st => info!("SAMPLE NV_PROM bulk read status {st:?}"),
    }

    info!("SAMPLE complete (read-only, nothing restored because nothing touched)");
}
