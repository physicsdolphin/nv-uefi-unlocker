//! ffspack CLI: turn the cargo-built `nvunlock-dxe.efi` (PE subsystem 0x0A)
//! into `NvUnlockDxe.efi` (subsystem 0x0B) plus `NvUnlockDxe.ffs` for manual
//! firmware insertion. See docs/dxe-driver-ffs-feasibility.md.
//!
//! ```text
//! ffspack build <in.efi> <out_prefix>   # writes <out_prefix>.efi and <out_prefix>.ffs
//! ffspack verify <file.ffs>
//! ```

use std::process::ExitCode;

use ffspack::{
    build_ffs, guid_bytes, pe_info, patch_subsystem, verify_ffs, NVUNLOCK_DXE_GUID,
    STATE_ERASE_POLARITY_ONE,
};
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") if args.len() >= 3 => cmd_build(&args[1], &args[2]),
        Some("verify") if args.len() >= 2 => cmd_verify(&args[1]),
        _ => {
            eprintln!(
                "usage:\n  ffspack build <in.efi> <out_prefix>\n  ffspack verify <file.ffs>"
            );
            ExitCode::FAILURE
        }
    }
}

fn cmd_build(input: &str, prefix: &str) -> ExitCode {
    let mut pe = match std::fs::read(input) {
        Ok(p) => p,
        Err(e) => return fail(&format!("read {input}: {e}")),
    };
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
    println!("PE:   subsystem patched -> {:#06x} (EFI_BOOT_SERVICE_DRIVER)", 0x0B);

    let guid = match guid_bytes(NVUNLOCK_DXE_GUID) {
        Ok(g) => g,
        Err(e) => return fail(&e),
    };
    let ffs = match build_ffs(&pe, &guid, "NvUnlockDxe", STATE_ERASE_POLARITY_ONE) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    if let Err(e) = verify_ffs(&ffs) {
        return fail(&format!("self-verification failed: {e}"));
    }

    let efi_out = format!("{prefix}.efi");
    let ffs_out = format!("{prefix}.ffs");
    if let Err(e) = std::fs::write(&efi_out, &pe) {
        return fail(&format!("write {efi_out}: {e}"));
    }
    if let Err(e) = std::fs::write(&ffs_out, &ffs) {
        return fail(&format!("write {ffs_out}: {e}"));
    }
    println!("WROTE {} ({} bytes)", efi_out, pe.len());
    println!("WROTE {} ({} bytes)", ffs_out, ffs.len());
    println!("SHA256 {} =", ffs_out);
    let mut h = Sha256::new();
    h.update(&ffs);
    for b in h.finalize() {
        print!("{b:02x}");
    }
    println!();
    println!("NOTE: the FFS carries no NVIDIA assets - markers/assets stay on the volume.");
    ExitCode::SUCCESS
}

fn cmd_verify(path: &str) -> ExitCode {
    let ffs = match std::fs::read(path) {
        Ok(f) => f,
        Err(e) => return fail(&format!("read {}: {e}", path)),
    };
    match verify_ffs(&ffs) {
        Ok(s) => {
            println!("FFS OK: type={:#04x} state={:#04x} sections={:?}", s.file_type, s.state, s.sections);
            println!("wrapped PE subsystem = {:?}", s.pe_subsystem.map(|v| format!("{v:#06x}")));
            ExitCode::SUCCESS
        }
        Err(e) => fail(&format!("{path}: {e}")),
    }
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("ffspack: {msg}");
    ExitCode::FAILURE
}
