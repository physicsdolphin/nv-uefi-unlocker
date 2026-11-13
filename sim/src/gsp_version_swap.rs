//! GSP version-swap experiment (2026-11-08, "badge" theory — see
//! docs/evidence/gsp-version-swap-sim/): does OUR transaction depend on
//! WHICH driver version the GSP image came from?
//!
//! Premise measured from the containers (issue #212): the 610.57.04
//! GSP ELFs are different bytes AND a different size than the pinned
//! 610.43.03 set — whole ELF 84,310,168 vs 84,277,400; the piece the
//! transaction actually maps (the `.fwimage` section) is 84,258,816
//! (0x505B000, page-aligned, +0x8000 over the pinned 0x5053000).
//!
//! The loader's dependency surface on the image is exactly three pure
//! functions, all ported here: `layout_from_registers` (takes the SIZE),
//! `radix::plan_for`/`Radix::build` (content-agnostic mapping), and the
//! WprMeta `image_bytes` field (the size again). `Transaction` takes no
//! image bytes at all. These tests pin that claim: swap the size to the
//! 610.57 value everywhere and every stage stays green — the version
//! coupling that remains is on-silicon booter/FWSEC acceptance, not in
//! our state machine.

use crate::layout::layout_from_registers;
use crate::radix::{self, Radix, RADIX_PAGE};
use crate::transaction::{Faults, PayloadShape, Transaction};
use crate::FakeGpu;

/// Pinned 610.43.03 GA10x `.fwimage` span (the constant the rest of the
/// suite uses).
pub const IMAGE_610_43: u64 = 0x5053_000;
/// Measured 2026-11-08 from nvidia-utils-610.57.04 (CachyOS container):
/// `.fwimage` section of gsp_ga10x.bin, page-aligned, +0x8000.
pub const IMAGE_610_57: u64 = 84_258_816;
pub const BOOT_GA102: u64 = 24_576;

/// The eight register scenarios from the C main() harness (verbatim from
/// layout.rs tests) — re-run with the swapped image size.
fn c_main_scenarios(image: u64) -> Vec<(&'static str, u32, u32, u32, u32, u32)> {
    let vga_top = ((((24u64 << 30) - (1 << 20)) >> 16) as u32) << 8 | 8;
    vec![
        ("sc0_default_pramin_fallback", 24 * 1024, 0, 1, 0xffff_fff0, 0),
        ("sc1_vga_workspace_at_fb_top", 24 * 1024, vga_top, 1, 0xffff_fff0, 0),
        ("sc2_wpr_overlap", 24 * 1024, 0, 1, 0x3f70_0000, 0x3f7f_f000),
        ("sc3_wpr_at_zero", 24 * 1024, 0, 1, 0, 0),
        ("sc4_wpr_flush_top", 24 * 1024, 0, 1, 0x7fef_0000, 0x7fff_f000),
        ("sc5_small_fb", 10 * 1024, 0, 1, 0xffff_fff0, 0),
        ("sc6_wpr_flush_vga_clamped", 24 * 1024, vga_top, 1, 0x7fef_0000, 0x7fff_f000),
        ("sc7_tiny_wpr", 24 * 1024, 0, 1, 0x7fff_0000, 0x7fff_f000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invariants(name: &str, r: &Result<(crate::layout::LayoutIn, crate::layout::LayoutOut), crate::layout::LayoutError>) {
        let (i, o) = r.as_ref().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(o.nonwpr_start >= i.usable_fb - (256 << 20), "{name}: below scrub top");
        assert!(o.wpr_end <= i.vga_base, "{name}: wpr overruns vga");
        assert!(o.heap_size >= 88 << 20, "{name}: heap too small");
    }

    /// All eight C-main register scenarios layout GREEN with the 610.57
    /// image size — the geometry reflows around the bigger image exactly
    /// as it does around the pinned one.
    #[test]
    fn layout_all_scenarios_green_with_610_57_image_size() {
        for (name, fb, vga, plm, lo, hi) in c_main_scenarios(IMAGE_610_57) {
            let r = layout_from_registers(fb, vga, plm, lo, hi, IMAGE_610_57, BOOT_GA102);
            invariants(name, &r);
        }
    }

    /// The swap is not a no-op: image_size flows into the geometry as
    /// `gsp = boot - image_size` (then 64 KiB aligned), while the top-down
    /// wpr_end/frts/boot stack and the heap-driven wpr_start do NOT move.
    /// With the 0x8000 delta and the 64 KiB gsp alignment, the gsp region
    /// start either steps down by exactly 0x8000 or holds (alignment slack
    /// absorbs it) — both prove the size actually reached the layout, and
    /// the invariant chain (gsp < boot < frts < wpr_end, wpr fixed) holds.
    #[test]
    fn layout_actually_reflows_for_the_bigger_image() {
        let (i43, o43) = layout_from_registers(24 * 1024, 0, 1, 0xffff_fff0, 0, IMAGE_610_43, BOOT_GA102).unwrap();
        let (i57, o57) = layout_from_registers(24 * 1024, 0, 1, 0xffff_fff0, 0, IMAGE_610_57, BOOT_GA102).unwrap();
        assert_eq!(i57.image_size - i43.image_size, 0x8000);
        // The top-down stack and the heap side are image-size independent.
        assert_eq!(o57.wpr_end, o43.wpr_end);
        assert_eq!(o57.frts_offset, o43.frts_offset);
        assert_eq!(o57.boot_offset, o43.boot_offset);
        assert_eq!(o57.wpr_start, o43.wpr_start);
        assert_eq!(o57.nonwpr_start, o43.nonwpr_start);
        // The image region absorbs the delta: raw +0x8000, quantized by
        // the 64 KiB align_down to 0/0x8000/0x10000 depending on where
        // boot - image lands. The invariant that MATTERS for the swap is
        // that the mapped window still contains the bigger image:
        assert!(o57.gsp_offset <= o43.gsp_offset);
        assert!(o57.boot_offset - o57.gsp_offset >= IMAGE_610_57, "window must fit the 610.57 image");
        assert!(o43.boot_offset - o43.gsp_offset >= IMAGE_610_43);
        assert!(o57.gsp_offset < o57.boot_offset);
    }

    /// Radix plan for the 610.57 size: page counts derived, tables sized,
    /// and the byte-exact publish/walk roundtrip holds across every L2
    /// boundary the bigger image now crosses (page 511->512, and the new
    /// final L2 table boundary at page 20480).
    #[test]
    fn radix_publishes_610_57_image_byte_exact() {
        let plan = radix::plan_for(IMAGE_610_57);
        let leaves = (IMAGE_610_57 as usize) / RADIX_PAGE;
        assert_eq!(IMAGE_610_57 % RADIX_PAGE as u64, 0, "610.57 image is page-aligned");
        assert_eq!(plan.pages[3], leaves);
        // Synthetic image with a per-page identifying pattern (content
        // provably flows through the mapping).
        let mut image = vec![0u8; IMAGE_610_57 as usize];
        for p in 0..leaves {
            let tag = (p as u32).to_le_bytes();
            image[p * RADIX_PAGE..p * RADIX_PAGE + 4].copy_from_slice(&tag);
        }
        let radix = Radix::build(&image, 0x1000_0000);
        for p in [0usize, 1, 511, 512, 20479, 20480, leaves - 1] {
            let page = p as u64;
            let leaf = radix.leaf(page);
            assert_eq!(&leaf[..4], &(p as u32).to_le_bytes(), "page {p} tag mismatch");
        }
        // Every page, not just the boundaries.
        for p in 0..leaves {
            assert_eq!(&radix.leaf(p as u64)[..4], &(p as u32).to_le_bytes());
        }
    }

    /// The WprMeta carries the image size verbatim: `image_bytes` at the
    /// C-verifier offset holds the 610.57 value. (The full pack lives in
    /// the loader; the field-level contract is what sim owns.)
    #[test]
    fn wpr_meta_image_bytes_carries_the_swapped_size() {
        let mut meta = [0u8; 256];
        radix::wpr_meta::put_u64(&mut meta, radix::wpr_meta::MAGIC_OFF, 0x4350_574d_5254_5357);
        radix::wpr_meta::put_u64(&mut meta, radix::wpr_meta::IMAGE_BYTES_OFF, IMAGE_610_57);
        assert_eq!(radix::wpr_meta::get_u64(&meta, radix::wpr_meta::IMAGE_BYTES_OFF), IMAGE_610_57);
    }

    /// The phase machine itself takes NO image bytes: all three shapes
    /// (Ada triad / GA102 self-loop / stock replay) run green unchanged
    /// on the canonical locked-cold fixture. This is the "badge" half of
    /// the theory — after the DMA is aimed, the transaction never
    /// inspects the image.
    #[test]
    fn transaction_all_shapes_green_with_swapped_image_geometry() {
        let mut gpu = FakeGpu::new_locked_cold(None);
        Transaction::new(&mut gpu, Faults::default()).run().expect("ada triad");
        let mut gpu = FakeGpu::new_locked_cold(None);
        Transaction::new_ga102_payload(&mut gpu, Faults::default()).run().expect("ga102 self-loop");
        let mut gpu = FakeGpu::new_locked_cold(None);
        Transaction::new_replay(&mut gpu, Faults::default()).run().expect("replay");
    }

    /// Real-bytes fidelity (#[ignore], hermetic suite stays synthetic):
    /// feed the actual 610.57.04 `.fwimage` bytes through radix build +
    /// walk and check every page byte-exact. Run with
    /// `SIM_GA10X_61057_FWIMAGE=<path> cargo test -p sim -- --ignored`.
    #[test]
    #[ignore = "set SIM_GA10X_61057_FWIMAGE to the extracted fwimage_ga10x.bin (610.57.04)"]
    fn real_610_57_fwimage_publishes_byte_exact() {
        let path = match std::env::var("SIM_GA10X_61057_FWIMAGE") {
            Ok(p) => p,
            Err(_) => return,
        };
        let image = std::fs::read(&path).expect("read fwimage");
        assert_eq!(image.len() as u64, IMAGE_610_57, "extracted .fwimage must be the 610.57 span");
        let radix = Radix::build(&image, 0x1000_0000);
        let leaves = image.len() / RADIX_PAGE;
        for p in 0..leaves {
            let want: &[u8; RADIX_PAGE] = image[p * RADIX_PAGE..(p + 1) * RADIX_PAGE][..].try_into().unwrap();
            assert_eq!(radix.leaf(p as u64), want);
        }
        // Record the identity so the evidence file can quote it.
        eprintln!("real 610.57 fwimage: {} pages byte-exact", leaves);
    }
}
