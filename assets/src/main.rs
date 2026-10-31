//! CLI: generate | extract <source.tar.xz> <outdir> | fw <package.tar.zst> <outdir>

use std::path::{Path, PathBuf};

use assets::manifest::ga102_610 as pins;

fn extract(source: &Path, outdir: &Path) -> Result<(), String> {
    use assets::booter::{build_bundle, build_gsp_boot};
    use assets::manifest::sha256_hex;

    let booter_src = assets::archive::extract_member_xz(
        source,
        &format!("NVIDIA-kernel-module-source-{}/src/nvidia/generated/{}", "610.43.03", pins::BINDATA_BOOTER_LOAD),
    )
    .map_err(|e| e.to_string())?;
    if sha256_hex(&booter_src) != pins::BINDATA_BOOTER_LOAD_SHA256 {
        return Err("pinned generated source mismatch (booter)".into());
    }
    let boot_src = assets::archive::extract_member_xz(
        source,
        &format!("NVIDIA-kernel-module-source-{}/src/nvidia/generated/{}", "610.43.03", pins::BINDATA_GSP_RM_BOOT),
    )
    .map_err(|e| e.to_string())?;
    if sha256_hex(&boot_src) != pins::BINDATA_GSP_RM_BOOT_SHA256 {
        return Err("pinned generated source mismatch (gsp rm boot)".into());
    }

    let booter_text = String::from_utf8(booter_src).map_err(|_| "non-ascii generated source")?;
    let boot_text = String::from_utf8(boot_src).map_err(|_| "non-ascii generated source")?;

    let booter = build_bundle(&booter_text).map_err(|e| e.to_string())?;
    let (image, desc) = build_gsp_boot(&boot_text).map_err(|e| e.to_string())?;

    let blobs = [
        ("booter_load_ga102.bin", booter, pins::DERIVED_BOOTER_LOAD),
        ("gsp_boot_ga102.bin", image, pins::DERIVED_GSP_BOOT),
        ("gsp_boot_desc_ga102.bin", desc, pins::DERIVED_GSP_BOOT_DESC),
    ];
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    for (name, data, pinned) in blobs {
        let digest = sha256_hex(&data);
        if digest != pinned {
            return Err(format!("derived asset differs from pinned SHA-256: {name} ({digest})"));
        }
        std::fs::write(outdir.join(name), &data).map_err(|e| e.to_string())?;
        println!("derived {name}: {} bytes, SHA-256 verified", data.len());
    }
    Ok(())
}

fn fw(package: &Path, outdir: &Path) -> Result<(), String> {
    use assets::manifest::sha256_hex;

    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    for (path, pin, out_name) in [
        (pins::GSP_GA10X_PATH, pins::GSP_GA10X_SHA256, "gsp_ga10x.bin"),
        (pins::GSP_TU10X_PATH, pins::GSP_TU10X_SHA256, "gsp_tu10x.bin"),
    ] {
        let gsp = assets::archive::extract_member_zst(package, path).map_err(|e| e.to_string())?;
        let digest = sha256_hex(&gsp);
        if digest != pin {
            return Err(format!("{out_name} SHA-256 mismatch ({digest})"));
        }
        std::fs::write(outdir.join(out_name), &gsp).map_err(|e| e.to_string())?;
        println!("{out_name}: {} bytes, SHA-256 verified", gsp.len());

        // Section inventory (identity record; sections themselves stay out of git).
        for s in assets::elf::sections(&gsp).map_err(|e| e.to_string())? {
            println!("  {out_name} section {} size {:#x}", s.name, s.size);
        }
    }
    Ok(())
}

fn vbios(rom: &Path) -> Result<(), String> {
    use assets::vbios;

    let raw = std::fs::read(rom).map_err(|e| format!("read: {e}"))?;
    // Some dump tools wrap the image (e.g. NVGI header); locate the real
    // option ROM start: 0xAA55 with a PCIR data structure at +0x18.
    let start = if raw.len() > 2 && &raw[0..2] == b"\x55\xaa" {
        0
    } else {
        (0..raw.len().saturating_sub(0x1000).min(0x1_0000))
            .step_by(0x200)
            .chain([0usize])
            .find(|&off| {
                raw.get(off..off + 2) == Some(b"\x55\xaa")
                    && off + 0x20 <= raw.len()
                    && raw[off + 0x18] % 4 == 0
            })
            .or_else(|| raw.windows(2).position(|w| w == b"\x55\xaa"))
            .ok_or("not a VBIOS image (no 0xAA55 found)")?
    };
    if start > 0 {
        println!("dump prefix: {start:#x} bytes stripped (tool wrapper)");
    }
    let img = &raw[start..];
    println!(
        "rom: {} bytes, first-64KiB crc32 {:#010x} (compare against M1 NV_PROM sample)",
        img.len(),
        crc32fast::hash(&img[..img.len().min(0x10000)])
    );
    let bit = vbios::find_bit_header(img).map_err(|e| e.to_string())?;
    let expansion = vbios::find_expansion_offset(img).map_err(|e| e.to_string())?;
    println!("BIT header @ {bit:#x}, expansion image @ {expansion:#x}");
    let (entries, _, table) = vbios::find_ucode_table(img).map_err(|e| e.to_string())?;
    println!("ucode table @ {table:#x}: {} entries", entries.len());
    for e in &entries {
        let name = match e.app_id {
            0x01 => "PRE_OS",
            0x05 => "FIRMWARE_SEC_LIC",
            0x08 => "LS_UDE",
            0x45 => "FWSEC_DBG",
            0x85 => "FWSEC_PROD",
            _ => "?",
        };
        let desc = vbios::parse_desc(img, expansion, e);
        let extra = match desc {
            Ok(vbios::Desc::V1(d)) => format!(
                "V1{} stored {:#x} imem {:#x} dmem {:#x}",
                if d.encrypted { " [ENCRYPTED]" } else { "" },
                d.stored_size, d.imem_load_size, d.dmem_load_size
            ),
            Ok(vbios::Desc::V2(d)) => format!(
                "V2 stored {:#x} imem {:#x} dmem {:#x} sec {:#x}+{:#x}",
                d.stored_size, d.imem_load_size, d.dmem_load_size, d.imem_sec_base, d.imem_sec_size
            ),
            Ok(vbios::Desc::V3(d)) => format!(
                "V3 stored {:#x} imem {:#x} dmem {:#x} sigs {} versions {:#x}",
                d.stored_size, d.imem_load_size, d.dmem_load_size, d.signature_count, d.signature_versions
            ),
            Err(err) => format!("desc error: {err}"),
        };
        println!("  app {:#04x} {} target {:#04x} {extra}", e.app_id, name, e.target_id);
    }
    Ok(())
}

/// Proven BOOT_DIRECT bootloader slice: verify the cmp50hx GspRmBoot debug
/// archive (pinned sha) and emit its first 0x6000 bytes as gsp_boot_tu10x.bin
/// — the exact image our TU106 unlock fired with, and the same input the
/// 50HX/40HX/chenxluo Windows runs fed the booter.
fn slice_proven_rm_boot(blob: &Path, outdir: &Path) -> Result<(), String> {
    use assets::manifest::{sha256_hex, tu10x_610 as pins};
    let data = std::fs::read(blob).map_err(|e| format!("read: {e}"))?;
    let src_digest = sha256_hex(&data);
    if src_digest != pins::PROVEN_RM_BOOT_SLICE_SRC_SHA256 {
        return Err(format!(
            "proven GspRmBoot blob mismatch: got {src_digest} want {} (pinned cmp50hx gsp_rm_boot_dbg.bin only)",
            pins::PROVEN_RM_BOOT_SLICE_SRC_SHA256
        ));
    }
    let slice = &data[..pins::PROVEN_RM_BOOT_SLICE_LEN];
    let digest = sha256_hex(slice);
    if digest != pins::PROVEN_RM_BOOT_SLICE_SHA256 {
        return Err(format!("proven slice mismatch: {digest}"));
    }
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    std::fs::write(outdir.join("gsp_boot_tu10x.bin"), slice).map_err(|e| e.to_string())?;
    println!(
        "gsp_boot_tu10x.bin: {} bytes, SHA-256 verified (proven BL slice, fired on TU106 2026-10-27)",
        slice.len()
    );
    Ok(())
}

fn extract_tu10x(source: &Path, outdir: &Path) -> Result<(), String> {
    use assets::booter::{build_bundle_direct, build_gsp_boot_v4};
    use assets::manifest::{sha256_hex, tu10x_610 as pins};
    let root = "NVIDIA-kernel-module-source-610.43.03/src/nvidia/generated/";
    let write_derived = |name: &str, data: &[u8], pinned: &str| -> Result<(), String> {        let digest = sha256_hex(data);
        if pinned != "UNMEASURED" && digest != pinned {
            return Err(format!("TU102 derived asset mismatch: {name} ({digest})"));
        }
        std::fs::write(outdir.join(name), data).map_err(|e| e.to_string())?;
        if pinned == "UNMEASURED" {
            println!("PIN-ME {name}: {} bytes, sha256 {digest}", data.len());
        } else {
            println!("TU102 derived {name}: {} bytes, SHA-256 verified", data.len());
        }
        Ok(())
    };
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    let booter_src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::BINDATA_BOOTER_LOAD))
        .map_err(|e| e.to_string())?;
    if sha256_hex(&booter_src) != pins::BINDATA_BOOTER_LOAD_SHA256 {
        return Err("pinned TU102 generated source mismatch (booter)".into());
    }
    let boot_src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::BINDATA_GSP_RM_BOOT))
        .map_err(|e| e.to_string())?;
    if sha256_hex(&boot_src) != pins::BINDATA_GSP_RM_BOOT_SHA256 {
        return Err("pinned TU102 generated source mismatch (gsp rm boot)".into());
    }
    let booter = build_bundle_direct(&String::from_utf8(booter_src.clone()).map_err(|_| "non-ascii")?)
        .map_err(|e| e.to_string())?;
    let (image, desc) = build_gsp_boot_v4(&String::from_utf8(boot_src.clone()).map_err(|_| "non-ascii")?)
        .map_err(|e| e.to_string())?;
    // Generic falcon BL (WITH_LOADER loader for the Turing FWSEC fire).
    let bl_src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::BINDATA_SEC2_BL))
        .map_err(|e| e.to_string())?;
    if pins::BINDATA_SEC2_BL_SHA256 != "UNMEASURED" && sha256_hex(&bl_src) != pins::BINDATA_SEC2_BL_SHA256 {
        return Err("pinned TU102 generated source mismatch (sec2 bl)".into());
    }
    let bl = assets::bindata::array_bytes(
        &String::from_utf8(bl_src.clone()).map_err(|_| "non-ascii")?,
        "UCODE_IMAGE",
    )
    .map_err(|e| e.to_string())?
    .data;
    // Raw generated sources stay on disk for forensics (never enter git).
    std::fs::write(outdir.join("booter_load_tu10x_raw.c"), &booter_src).map_err(|e| e.to_string())?;
    std::fs::write(outdir.join("gsp_boot_tu10x_raw.c"), &boot_src).map_err(|e| e.to_string())?;
    std::fs::write(outdir.join("sec2_bl_tu10x_raw.c"), &bl_src).map_err(|e| e.to_string())?;
    write_derived("booter_load_tu10x.bin", &booter, pins::DERIVED_BOOTER_LOAD)?;
    write_derived("gsp_boot_tu10x.bin", &image, pins::DERIVED_GSP_BOOT)?;
    write_derived("gsp_boot_desc_tu10x.bin", &desc, pins::DERIVED_GSP_BOOT_DESC)?;
    write_derived("sec2_bl_tu10x.bin", &bl, pins::DERIVED_SEC2_BL)?;
    Ok(())
}

fn extract_scrub(source: &Path, outdir: &Path) -> Result<(), String> {
    use assets::booter::build_bundle;
    use assets::manifest::{ad102_610 as pins, sha256_hex};
    let root = "NVIDIA-kernel-module-source-610.43.03/src/nvidia/generated/";
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    let src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::SCRUB_BINDATA))
        .map_err(|e| e.to_string())?;
    if sha256_hex(&src) != pins::SCRUB_BINDATA_SHA256 {
        return Err("pinned SEC2 scrubber generated source mismatch".into());
    }
    let bundle = build_bundle(&String::from_utf8(src.clone()).map_err(|_| "non-ascii")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(outdir.join("scrubber_ad10x_raw.c"), &src).map_err(|e| e.to_string())?;
    let digest = sha256_hex(&bundle);
    if pins::SCRUB_DERIVED != "UNMEASURED" && digest != pins::SCRUB_DERIVED {
        return Err(format!("SEC2 scrubber derived mismatch: {digest}"));
    }
    std::fs::write(outdir.join("scrubber_ad10x.bin"), &bundle).map_err(|e| e.to_string())?;
    if pins::SCRUB_DERIVED == "UNMEASURED" {
        println!("PIN-ME scrubber_ad10x.bin: {} bytes, sha256 {digest}", bundle.len());
    } else {
        println!("scrubber_ad10x.bin: {} bytes, SHA-256 verified", bundle.len());
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: assets <generate [out.bin] | generate-ad102 [out.bin] | generate-tu10x [out.bin] | extract <src.tar.xz> <outdir> | extract-ad102 <src.tar.xz> <outdir> | extract-tu10x <src.tar.xz> <outdir> | extract-scrub <src.tar.xz> <outdir> | slice-proven-rm-boot <gsp_rm_boot_dbg.bin> <outdir> | fw <pkg.tar.zst> <outdir> | gsp-sections <gsp.bin> <outdir> | vbios <rom>>";
    let result = match args.get(1).map(String::as_str) {
        Some("generate") => {
            let out: PathBuf = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "v67-ga102-610.43.03.bin".into())
                .into();
            assets::payload::build_pinned_ga102().and_then(|payload| {
                std::fs::write(&out, &payload).map_err(|e| e.to_string())?;
                println!("wrote {} ({} bytes, SHA-256 verified)", out.display(), payload.len());
                Ok(())
            })
        }
        Some("generate-ad102") => {
            let out: PathBuf = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "v67-ad102-610.43.03-four-plm.bin".into())
                .into();
            assets::payload::build_pinned_ad102().and_then(|payload| {
                std::fs::write(&out, &payload).map_err(|e| e.to_string())?;
                println!("wrote {} ({} bytes, SHA-256 verified)", out.display(), payload.len());
                Ok(())
            })
        }
        Some("generate-tu10x") => {
            let out: PathBuf = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "v67-tu10x-four-plm.bin".into())
                .into();
            assets::payload::build_pinned_tu102().and_then(|payload| {
                std::fs::write(&out, &payload).map_err(|e| e.to_string())?;
                println!("wrote {} ({} bytes, SHA-256 verified)", out.display(), payload.len());
                Ok(())
            })
        }
        Some("extract") => match (args.get(2), args.get(3)) {
            (Some(s), Some(o)) => extract(Path::new(s), Path::new(o)),
            _ => Err(usage.to_string()),
        },
        Some("fw") => match (args.get(2), args.get(3)) {
            (Some(s), Some(o)) => fw(Path::new(s), Path::new(o)),
            _ => Err(usage.to_string()),
        },
        Some("extract-tu10x") => match (args.get(2), args.get(3)) {
            (Some(s), Some(o)) => extract_tu10x(Path::new(s), Path::new(o)),
            _ => Err(usage.to_string()),
        },
        // slice-proven-rm-boot <gsp_rm_boot_dbg.bin> <outdir>
        // Verifies the pinned cmp50hx GspRmBoot debug archive and writes the
        // proven 24KiB bootloader slice as gsp_boot_tu10x.bin.
        Some("slice-proven-rm-boot") => match (args.get(2), args.get(3)) {
            (Some(s), Some(o)) => slice_proven_rm_boot(Path::new(s), Path::new(o)),
            _ => Err(usage.to_string()),
        },
        Some("extract-scrub") => match (args.get(2), args.get(3)) {
            (Some(src), Some(out)) => extract_scrub(Path::new(src), Path::new(out)),
            _ => Err(usage.to_string()),
        },
        Some("extract-ad102") => match (args.get(2), args.get(3)) {
            (Some(src), Some(out)) => extract_ad102(Path::new(src), Path::new(out)),
            _ => Err(usage.to_string()),
        },
        Some("gsp-sections") => match (args.get(2), args.get(3)) {
            (Some(src), Some(out)) => gsp_sections(Path::new(src), Path::new(out)),
            _ => Err(usage.to_string()),
        },
        Some("vbios") => match args.get(2) {
            Some(s) => vbios(Path::new(s)),
            _ => Err(usage.to_string()),
        },
        _ => Err(usage.to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }


fn extract_ad102(source: &Path, outdir: &Path) -> Result<(), String> {
    use assets::booter::{build_bundle, build_gsp_boot};
    use assets::manifest::{ad102_610 as pins, sha256_hex};
    let root = "NVIDIA-kernel-module-source-610.43.03/src/nvidia/generated/";
    let booter_src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::BINDATA_BOOTER_LOAD))
        .map_err(|e| e.to_string())?;
    if sha256_hex(&booter_src) != pins::BINDATA_BOOTER_LOAD_SHA256 {
        return Err("pinned AD102 generated source mismatch (booter)".into());
    }
    let boot_src = assets::archive::extract_member_xz(source, &format!("{root}{}", pins::BINDATA_GSP_RM_BOOT))
        .map_err(|e| e.to_string())?;
    if sha256_hex(&boot_src) != pins::BINDATA_GSP_RM_BOOT_SHA256 {
        return Err("pinned AD102 generated source mismatch (gsp rm boot)".into());
    }
    let booter = build_bundle(&String::from_utf8(booter_src).map_err(|_| "non-ascii")?)
        .map_err(|e| e.to_string())?;
    let (image, desc) = build_gsp_boot(&String::from_utf8(boot_src).map_err(|_| "non-ascii")?)
        .map_err(|e| e.to_string())?;
    let blobs = [
        ("booter_load_ad102.bin", booter, pins::DERIVED_BOOTER_LOAD),
        ("gsp_boot_ad102.bin", image, pins::DERIVED_GSP_BOOT),
        ("gsp_boot_desc_ad102.bin", desc, pins::DERIVED_GSP_BOOT_DESC),
    ];
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    for (name, data, pinned) in blobs {
        let digest = sha256_hex(&data);
        if digest != pinned {
            return Err(format!("AD102 derived asset mismatch: {name} ({digest})"));
        }
        std::fs::write(outdir.join(name), &data).map_err(|e| e.to_string())?;
        println!("AD102 derived {name}: {} bytes, SHA-256 verified", data.len());
    }
    Ok(())
}

fn gsp_sections(gsp_bin: &Path, outdir: &Path) -> Result<(), String> {
    let img = std::fs::read(gsp_bin).map_err(|e| format!("read: {e}"))?;
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    // Export EVERY family signature section plus the image, named for the
    // loader's per-family lookup (fwimage_<fam>.bin / fwsignature_<fam>.bin).
    for s in assets::elf::sections(&img).map_err(|e| e.to_string())? {
        if let Some(fam) = s.name.strip_prefix(".fwsignature_") {
            let data = assets::elf::section_bytes(&img, &s.name).map_err(|e| e.to_string())?;
            std::fs::write(outdir.join(format!("fwsignature_{fam}.bin")), data)
                .map_err(|e| e.to_string())?;
            println!("fwsignature_{fam}.bin: {} bytes", data.len());
        }
    }
    let fwimage = assets::elf::section_bytes(&img, ".fwimage").map_err(|e| e.to_string())?;
    // The image itself is family-neutral; export one per signature family
    // present so each family dir is self-contained.
    for s in assets::elf::sections(&img).map_err(|e| e.to_string())? {
        if let Some(fam) = s.name.strip_prefix(".fwsignature_") {
            std::fs::write(outdir.join(format!("fwimage_{fam}.bin")), fwimage)
                .map_err(|e| e.to_string())?;
            println!("fwimage_{fam}.bin: {} bytes (same image)", fwimage.len());
        }
    }
    Ok(())
}
}