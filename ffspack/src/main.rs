//! ffspack CLI: turn the cargo-built `nvunlock-dxe.efi` (PE subsystem 0x0A)
//! into `NvUnlockDxe.efi` (subsystem 0x0B) plus `NvUnlockDxe.ffs` for manual
//! firmware insertion. See docs/dxe-driver-ffs-feasibility.md.
//!
//! ```text
//! ffspack build <in.efi> <out_prefix> [--state 0xF8] [--force]
//! ffspack verify <file.ffs> [--expect-sha256 <hex>]
//! ```
//!
//! Both outputs are staged and renamed into place (audit M5); existing
//! outputs are refused without `--force`; both artifact hashes are printed
//! (the `.efi` is what the zero-flash shell-load path actually loads).

use std::process::ExitCode;

use ffspack::{
    build_ffs, guid_bytes, has_build_identity, patch_subsystem, pe_info, verify_nvunlock_dxe,
    DXE_BUILD_TAG, NVUNLOCK_DXE_GUID, STATE_ERASE_POLARITY_ONE,
};
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") => cmd_build(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        _ => {
            eprintln!(
                "usage:\n  ffspack build <in.efi> <out_prefix> [--state 0xF8] [--force]\n  ffspack verify <file.ffs> [--expect-sha256 <hex>]"
            );
            ExitCode::FAILURE
        }
    }
}

struct Args {
    positional: Vec<String>,
    state: u8,
    force: bool,
    expect_sha256: Option<String>,
}

fn parse_args(rest: &[String]) -> Result<Args, String> {
    let mut out = Args { positional: Vec::new(), state: STATE_ERASE_POLARITY_ONE, force: false, expect_sha256: None };
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--state" => {
                i += 1;
                let v = rest.get(i).ok_or("--state needs a value")?;
                out.state = u8::from_str_radix(v.trim_start_matches("0x"), 16)
                    .map_err(|e| format!("--state {v}: {e}"))?;
            }
            "--force" => out.force = true,
            "--expect-sha256" => {
                i += 1;
                let v = rest.get(i).ok_or("--expect-sha256 needs a value")?;
                out.expect_sha256 = Some(v.to_lowercase());
            }
            other => out.positional.push(other.to_string()),
        }
        i += 1;
    }
    Ok(out)
}

fn cmd_build(rest: &[String]) -> ExitCode {
    let args = match parse_args(rest) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let [input, prefix] = args.positional.as_slice() else {
        return fail("build needs exactly two arguments: <in.efi> <out_prefix>");
    };

    let mut pe = match std::fs::read(input) {
        Ok(p) => p,
        Err(e) => return fail(&format!("read {input}: {e}")),
    };
    if !has_build_identity(&pe) {
        return fail(&format!(
            "{input} does not carry the DXE build-identity tag '{}' - refusing (audit F4: never pack the USB application into firmware)",
            String::from_utf8_lossy(DXE_BUILD_TAG)
        ));
    }
    let before = match pe_info(&pe) {
        Ok(i) => i,
        Err(e) => return fail(&format!("{input}: {e}")),
    };
    println!(
        "PE:   machine={:#06x} subsystem={:#06x} reloc rva={:#x} size={:#x}",
        before.machine, before.subsystem, before.reloc_rva, before.reloc_size
    );
    if let Err(e) = patch_subsystem(&mut pe) {
        return fail(&e);
    }
    println!("PE:   subsystem patched -> 0x000b (EFI_BOOT_SERVICE_DRIVER)");

    let guid = match guid_bytes(NVUNLOCK_DXE_GUID) {
        Ok(g) => g,
        Err(e) => return fail(&e),
    };
    let ffs = match build_ffs(&pe, &guid, "NvUnlockDxe", args.state) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    if let Err(e) = verify_nvunlock_dxe(&ffs) {
        return fail(&format!("self-verification failed: {e}"));
    }

    let efi_out = format!("{prefix}.efi");
    let ffs_out = format!("{prefix}.ffs");
    for out in [&efi_out, &ffs_out] {
        if std::path::Path::new(out).exists() && !args.force {
            return fail(&format!("{out} exists - use --force to overwrite (audit M5: a re-run must not destroy a known-good set silently)"));
        }
    }

    // Stage both outputs, then move them into place (audit M5).
    let efi_tmp = format!("{efi_out}.tmp");
    let ffs_tmp = format!("{ffs_out}.tmp");
    if let Err(e) = std::fs::write(&efi_tmp, &pe) {
        return fail(&format!("write {efi_tmp}: {e}"));
    }
    if let Err(e) = std::fs::write(&ffs_tmp, &ffs) {
        let _ = std::fs::remove_file(&efi_tmp);
        return fail(&format!("write {ffs_tmp}: {e}"));
    }
    for (tmp, out) in [(&efi_tmp, &efi_out), (&ffs_tmp, &ffs_out)] {
        if std::path::Path::new(out).exists() {
            if let Err(e) = std::fs::remove_file(out) {
                let _ = std::fs::remove_file(tmp);
                return fail(&format!("replace {out}: {e}"));
            }
        }
        if let Err(e) = std::fs::rename(tmp, out) {
            return fail(&format!("rename {tmp} -> {out}: {e}"));
        }
    }

    println!("WROTE {} ({} bytes)", efi_out, pe.len());
    println!("WROTE {} ({} bytes)", ffs_out, ffs.len());
    for (name, bytes) in [(&efi_out, &pe), (&ffs_out, &ffs)] {
        let mut h = Sha256::new();
        h.update(bytes);
        print!("SHA256 {name} = ");
        for b in h.finalize() {
            print!("{b:02x}");
        }
        println!();
    }
    println!("NOTE: erase polarity of the built State byte = {:#04x} - must match the target FV (audit N10); UEFITool normalizes on insert.", args.state);
    println!("NOTE: the FFS carries no NVIDIA assets - markers/assets stay on the volume.");
    ExitCode::SUCCESS
}

fn cmd_verify(rest: &[String]) -> ExitCode {
    let args = match parse_args(rest) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    let [path] = args.positional.as_slice() else {
        return fail("verify needs exactly one argument: <file.ffs>");
    };
    let ffs = match std::fs::read(path) {
        Ok(f) => f,
        Err(e) => return fail(&format!("read {path}: {e}")),
    };
    match verify_nvunlock_dxe(&ffs) {
        Ok(s) => {
            println!(
                "FFS OK: type={:#04x} state={:#04x} data_checksum_ok={} sections={:?}",
                s.file_type, s.state, s.data_checksum_ok, s.sections
            );
            println!("wrapped PE subsystem = {:?}", s.pe_subsystem.map(|v| format!("{v:#06x}")));
            let mut h = Sha256::new();
            h.update(&ffs);
            let digest: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
            println!("SHA256 {path} = {digest}");
            if let Some(expect) = &args.expect_sha256 {
                if &digest != expect {
                    return fail(&format!("SHA256 mismatch: expected {expect}"));
                }
                println!("SHA256 matches --expect-sha256");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail(&format!("{path}: {e}")),
    }
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("ffspack: {msg}");
    ExitCode::FAILURE
}
