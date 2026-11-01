//! Full-corpus compatibility sweep over the local VBIOS collection.
//!
//! Gated by `ROMCORE_VBIOS_CORPUS` (a directory of *.rom dumps) so normal
//! CI never depends on NVIDIA firmware. For every ROM this exercises the
//! exact parser the loader's native-GOP path uses: start location (dump
//! wrappers), PCIR chain walk, code-type-0x03 selection, EFI-ROM header
//! fields, payload bounds. Prints one verdict line per ROM.

use romcore::efirom;
use romcore::walk;

#[test]
fn corpus_sweep() {
    let Ok(dir) = std::env::var("ROMCORE_VBIOS_CORPUS") else {
        eprintln!("skipped: set ROMCORE_VBIOS_CORPUS=<dir of *.rom> to run the sweep");
        return;
    };
    let mut roms: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("corpus dir {dir}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rom").unwrap_or(false))
        .collect();
    roms.sort();
    assert!(!roms.is_empty(), "corpus dir has no *.rom");
    let (mut ok, mut skip, mut fail) = (0usize, 0usize, 0usize);
    for p in &roms {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let raw = match std::fs::read(p) {
            Ok(r) => r,
            Err(e) => {
                println!("CORPUS {name}: READ-FAIL {e}");
                fail += 1;
                continue;
            }
        };
        let start = walk::locate_start(&raw);
        let img = &raw[start..];
        if img.len() < 0x40 {
            println!("CORPUS {name}: ROM-TOO-SMALL");
            fail += 1;
            continue;
        }
        match walk::efi_images(img) {
            Err(e) => {
                println!("CORPUS {name}: WALK-FAIL {e}");
                fail += 1;
            }
            Ok(efis) if efis.is_empty() => {
                println!("CORPUS {name}: NO-EFI-IMAGE (pre-GOP or non-EFI ROM)");
                skip += 1;
            }
            Ok(efis) => {
                let mut verdict = String::from("OK");
                for (n, e) in efis.iter().enumerate() {
                    let image = &img[e.start..e.start + e.len];
                    match efirom::EfiRomHeader::parse(image) {
                        Err(err) => verdict = format!("HDR-FAIL({err})"),
                        Ok(h) => {
                            let payload = efirom::payload(image, &h);
                            let avail = e.len.saturating_sub(h.image_header_offset + 8);
                            match payload {
                                Err(err) => verdict = format!("PAYLOAD-FAIL({err})"),
                                Ok(pl) => {
                                    let bounds_ok = (pl.compressed_size as usize) <= avail;
                                    verdict = format!(
                                        "OK efi#{n}@{:#x} {}B sub={} mach={:#06x} comp={} payload {}->{} bounds={}",
                                        e.start, e.len, h.efi_subsystem, h.efi_machine_type,
                                        h.compression_type, pl.compressed_size,
                                        pl.uncompressed_size, bounds_ok
                                    );
                                }
                            }
                        }
                    }
                }
                println!("CORPUS {name}: {verdict}");
                ok += 1;
            }
        }
    }
    println!("CORPUS TOTAL: {} roms = {ok} parsed + {skip} no-efi + {fail} failures", roms.len());
}
