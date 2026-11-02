//! Pinned inputs and derived assets (ported from gpu-unlock
//! firmware/artifacts-610.43.03.json + generate_v67.py pins).
//!
//! Constitution §6: NVIDIA-derived blobs never enter git; everything
//! regenerates from these pinned public inputs and hard-fails on any
//! hash mismatch. When a pinned download fails you may NOT substitute
//! another version.

use sha2::{Digest, Sha256};

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// GA102 / 610.43.03 rejoin15 reference path (reference target for the
/// pipeline machinery; TU106 assets will be pinned separately from a
/// Turing-pinned driver per task brief §9.1).
pub mod ga102_610 {
    /// NVIDIA-kernel-module-source-610.43.03.tar.xz
    pub const SOURCE_ARCHIVE_SHA256: &str =
        "7e118923c7a23edc36114d63273a46e3e04e9af98695a42203e7ac2dfe9fc1dc";
    pub const SOURCE_ARCHIVE_URL: &str =
        "https://download.nvidia.com/XFree86/NVIDIA-kernel-module-source/NVIDIA-kernel-module-source-610.43.03.tar.xz";

    /// nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst (CachyOS archive)
    pub const UTILS_PACKAGE_SHA256: &str =
        "e17e7c92eeffa178b0fe1f40eb17a47095f8fc3ce4a77bf115cf5bd5a4df86e9";
    pub const UTILS_PACKAGE_URL: &str =
        "https://archive.cachyos.org/archive/cachyos-v3/nvidia-utils-610.43.03-1-x86_64_v3.pkg.tar.zst";
    pub const UTILS_PACKAGE_VERSION: &str = "610.43.03-1";
    pub const GSP_GA10X_PATH: &str = "usr/lib/firmware/nvidia/610.43.03/gsp_ga10x.bin";
    pub const GSP_GA10X_SHA256: &str =
        "572373620a37418f24dc16b5031c39338778c3257e48e8408de9a57291b24f3a";

    /// TU10x (Turing) GSP image — the TU106 asset source (task brief §9.1:
    /// Turing assets come from the same pinned driver, never from GA10x).
    /// Hash measured 2026-10-25 from the pinned package above.
    pub const GSP_TU10X_PATH: &str = "usr/lib/firmware/nvidia/610.43.03/gsp_tu10x.bin";
    pub const GSP_TU10X_SHA256: &str =
        "73065619db9ec921d19fc4e519dd04d91a9199b525eaca9b257b89fb8c5ec52c";

    /// Generated-source BINDATA files inside the source archive.
    pub const BINDATA_BOOTER_LOAD: &str = "g_bindata_kgspGetBinArchiveBooterLoadUcode_GA102.c";
    pub const BINDATA_BOOTER_LOAD_SHA256: &str =
        "04cd3db7f670adc4efd79d2e76217bb809fffddfbd6f1af884cde2af26dd39af";
    pub const BINDATA_GSP_RM_BOOT: &str = "g_bindata_kgspGetBinArchiveGspRmBoot_GA102.c";
    pub const BINDATA_GSP_RM_BOOT_SHA256: &str =
        "35e5aa9075ad3898b6022ffd5820c953d809caa7dabd530cfc1a83dc2a67c5bd";

    /// Derived assets (validated end-to-end after extraction).
    pub const DERIVED_BOOTER_LOAD: &str =
        "056cbc064a888f80717726c54d7f64fcd68044d123870c95a74f37f4975dcaa4";
    pub const DERIVED_GSP_BOOT: &str =
        "3df651acb174fd5f5561a1996965b192f50c5d763168446ed0d4ead19d895768";
    pub const DERIVED_GSP_BOOT_DESC: &str =
        "cc5a3fc1f4c47f2fe107b1b68f0176d8a448e6dec84b575e5560ccef5262bf15";

    /// rejoin15 V67 reference payload.
    pub const V67_PAYLOAD_SHA256: &str =
        "2a0aa1f86d0b6afff5214d641d87d46dc98ef06ed908ff05d0ddbad5ffb7e197";
}

/// AD102-family (Ada) pins from gpu-unlock-ada firmware/ada-610.43.03.json.
/// The AD102 booter serves the whole AD10x family (same driver archive
/// pattern as GA102 serving GA104/GA106); the four-write payload is
/// hardware-evidenced on AD102 4090 / AD103 4080 / AD106 4060Ti.
/// AD107M remains unverified by the community (our M1 data is the probe).
pub mod ad102_610 {
    pub const BINDATA_BOOTER_LOAD: &str = "g_bindata_kgspGetBinArchiveBooterLoadUcode_AD102.c";
    pub const BINDATA_BOOTER_LOAD_SHA256: &str =
        "0504f640bc36d03c3b617b1d80cf6eee1c8022cdfffb52ed22405aaa316a1460";
    pub const BINDATA_GSP_RM_BOOT: &str = "g_bindata_kgspGetBinArchiveGspRmBoot_AD102.c";
    pub const BINDATA_GSP_RM_BOOT_SHA256: &str =
        "da110719ed6b680f26f3f7158f6731d1ff20fa859857238e23ac4935bd4cd91c";

    pub const DERIVED_BOOTER_LOAD: &str =
        "6ee05b20e1dc244c08c052f36ef28469b8d09f4955d1fe33e48a179e11a7fd49";
    pub const DERIVED_GSP_BOOT: &str =
        "45d6335a8185d6ff13e8d1484cea73f68d3d13aa1c19dc080ed1c3482dd5bac0";
    pub const DERIVED_GSP_BOOT_DESC: &str =
        "83f8f3f1f5f9496e0ea8fc7103c2beb64d0b6de2f5ffe725f13514de5a95fa42";

    /// four-plm payload: single fire, four chained writes + mailbox marker.
    pub const V67_AD102_PAYLOAD_SHA256: &str =
        "44c36b2d6b3424e53e18f3e55db85c833655de071ce66ce42749588a4a7dc8cb";

    /// bar1-64g payload variant: same four-write chain, operands CYA 0x30 /
    /// CFG 0x8000000a / XVE PLM / FEAT PLM, marker 0x67b00004. SHA equals the
    /// gpu-unlock-bar1 bar1-64g-report.md published digest of the payload
    /// field-proven on eight 10de:2684 cards (2026-10-24) — byte-identical
    /// reproduction, hard-fail on drift.
    pub const V67_AD102_BAR1_64G_PAYLOAD_SHA256: &str =
        "cf3566d0887d3d31e8cb3611bf0a1b3fafcd9584ea7a73306f75f8d0eafb1284";

    /// Kernel-exact SEC2 scrubber (kgspAllocateScrubberUcodeImage ←
    /// ksec2GetBinArchiveSecurescrubUcode): image 7424B, ucode 0xd,
    /// 2×RSA-384. THIS is what advances the BSI scrubber handoff — not the
    /// VBIOS PROM app 0x49. Pins measured 2026-10-26 from the pinned
    /// archive; hard-fail on drift.
    pub const SCRUB_BINDATA: &str = "g_bindata_ksec2GetBinArchiveSecurescrubUcode_AD10X.c";
    pub const SCRUB_BINDATA_SHA256: &str =
        "94dbc82226634bf7d1cf6d00691a5a5389d0517c687e8c17aa533f527ad6f312";
    pub const SCRUB_DERIVED: &str =
        "1cfc78136a4dff4993b7e5a1a09345f6a67d0762555e5bd9660e406831379348";
}

/// TU102-family (Turing) pins, same pinned 610.43.03 sources as GA102.
/// Turing BooterLoad is KGSP_FLCN_UCODE_BOOT_DIRECT (bBootFromHs=FALSE,
/// kernel_gsp_tu102.c:79): one 16-byte patch signature, ucode id 0xd.
/// GspRmBoot is an RM_RISCV_UCODE_DESC **v4** (76 bytes). Derived pins
/// measured 2026-10-25 from the pinned archive below; hard-fail on drift.
pub mod tu10x_610 {
    pub const BINDATA_BOOTER_LOAD: &str = "g_bindata_kgspGetBinArchiveBooterLoadUcode_TU102.c";
    pub const BINDATA_BOOTER_LOAD_SHA256: &str =
        "e495483adf66798502667da137c4a24c55f471b86f3183020e3169ff88ad6c89";
    pub const BINDATA_GSP_RM_BOOT: &str = "g_bindata_kgspGetBinArchiveGspRmBoot_TU102.c";
    pub const BINDATA_GSP_RM_BOOT_SHA256: &str =
        "19628e0b148a5437b351ce9ecd4a04405ec4eb66521e1c318ca6ef5d1a0c2efe";
    /// Generic falcon BL ("works for both SEC2 and GSP"): the WITH_LOADER
    /// loader that DMAs FWSEC's secure code from host memory into secure
    /// IMEM (cmp50hx v70; host secure-IMEM writes are scrubbed).
    pub const BINDATA_SEC2_BL: &str = "g_bindata_ksec2GetBinArchiveBlUcode_TU102.c";
    pub const BINDATA_SEC2_BL_SHA256: &str =
        "fc0bc277eeee754ea3071ae0fe8360a52193565bba7c07dcea60b00e8d56d9c8";

    /// PROVEN BL image for the BOOT_DIRECT booter: the first 0x6000 bytes of
    /// cmp50hx-unlock's gsp_rm_boot_dbg.bin (GspRmBoot debug archive, 128KiB,
    /// extracted from the same pinned 610.43.03 driver; upstream MIT). The
    /// kernel BINDATA UCODE_IMAGE is only 4096B - too small for the
    /// bootloader the booter consumes; the 24KiB slice is what the 50HX/40HX
    /// and chenxluo TU102-22G Windows runs all fed the booter. Our TU106
    /// unlock fired with exactly this slice (de676bae...).
    pub const PROVEN_RM_BOOT_SLICE_SRC_SHA256: &str =
        "fa43239bcee7b97ca62f007cc68487560a39e19f74f3dde7486db3f98df8e471";
    pub const PROVEN_RM_BOOT_SLICE_LEN: usize = 0x6000;
    pub const PROVEN_RM_BOOT_SLICE_SHA256: &str =
        "de676bae28a480011d3d012db14bef539324e62a841a9627863c689bea168af3";

    pub const DERIVED_BOOTER_LOAD: &str =
        "51507de61777cf46f3703a084cdfa896c21f6bc0c8af8f245bd09b2e9c49bf54";
    pub const DERIVED_GSP_BOOT: &str =
        "8765694cd38beaa4e2b8b1b83a31fb7aee6cb9d8b7c6bc7a86e6d931a3a056c2";
    pub const DERIVED_GSP_BOOT_DESC: &str =
        "9e7cca2cfdc5ce601b0adba39967850046108df6c2d23fff95578ca3f0ccbe01";
    pub const DERIVED_SEC2_BL: &str = "UNMEASURED";

    /// TU102 V67 signature image, mutex-free variant — pinned from
    /// nlzy/turing-unlock firmware/artifacts-610.43.03.json
    /// ("tu102_payload", generator firmware/generate_v67.py; the same
    /// image is proven live by cmp50hx-unlock efi-unlock on 2026-09-10).
    /// Two chained writes (FEAT_PLM=0xffffffff, WPR2_HI=0) plus the ACR
    /// mutex-3 release tokens; fill 0x4a7, guard 0x6140.
    pub const V67_TU10X_PAYLOAD_SHA256: &str =
        "b0ec90294d1de3c67e024631c0a11000f281470d76eed806fc24c66b7fd1ef41";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_empty_matches_known_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
