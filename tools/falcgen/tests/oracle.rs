//! Oracle tests against envydis ground truth. Silent skip unless
//! `FALCGEN_SEC_DIR` points at a directory holding the decrypted booter
//! plaintexts + envydis lst files (the RE workspace; decrypted artifacts
//! never enter git):
//!
//!   TU102_sec_plain.bin / TU102_sec.lst   (also TU116/GA102/AD102/GA100)
//!   FALCGEN_ENVYDIS = WSL command prefix to the envydis binary, e.g.
//!     "wsl -e /mnt/d/git-repo/nv-uefi-unlocker/reference/envytools/envydis/envydis"
//!   (optional; enables the non-canonical-offset sampling check)

use falcgen::decode::{decode_linear, decode_one, parse_lst};
use falcgen::isa::{Variant, FUC5};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const DIES: [&str; 5] = ["TU102", "TU116", "GA102", "AD102", "GA100"];

fn sec_dir() -> Option<PathBuf> {
    let dir = std::env::var("FALCGEN_SEC_DIR").ok()?;
    let p = PathBuf::from(dir);
    p.is_dir().then_some(p)
}

/// envydis's mnemonic for the instruction at `off`: feed a 6-byte window
/// (maxoplen) via stdin, take the first line's first token.
fn envydis_mnemonic(envy: &str, window: &[u8]) -> String {
    let mut child = Command::new(envy.split_whitespace().next().unwrap())
        .args(envy.split_whitespace().skip(1))
        .args(["-m", "falcon", "-V", "fuc5", "-i", "-n", "-q"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn envydis");
    child.stdin.as_mut().unwrap().write_all(window).unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .next()
        .unwrap_or("???")
        .split_whitespace()
        .next()
        .unwrap_or("???")
        .to_string()
}

#[test]
fn canonical_stream_matches_envydis_lst() {
    let Some(dir) = sec_dir() else { return };
    for die in DIES {
        let plain = std::fs::read(dir.join(format!("{die}_sec_plain.bin")))
            .unwrap_or_else(|e| panic!("{die}: {e}"));
        let lst = std::fs::read_to_string(dir.join(format!("{die}_sec.lst")))
            .unwrap_or_else(|e| panic!("{die}: {e}"));
        let want = parse_lst(&lst);
        let got = decode_linear(&plain, 0x100, FUC5);
        assert_eq!(want.len(), got.len(), "{die}: instruction count");
        let first = want
            .iter()
            .zip(got.iter())
            .enumerate()
            .find(|(_, ((off, mn), insn))| insn.pos - 0x100 != *off || insn.name != *mn);
        assert!(
            first.is_none(),
            "{die}: first mismatch at index {:?}: lst {:?} vs ours {} ({})",
            first.map(|(i, _)| i),
            first.map(|(_, ((o, m), _))| (o, m)),
            first.map(|(_, (_, insn))| format!("{:x} {}", insn.pos - 0x100, insn.name)).unwrap_or_default(),
            first.map(|(_, (_, insn))| insn.render()).unwrap_or_default(),
        );
    }
}

#[test]
fn non_canonical_offsets_match_envydis() {
    let Some(dir) = sec_dir() else { return };
    let Ok(envy) = std::env::var("FALCGEN_ENVYDIS") else { return };
    // Sample every 97th offset of TU102 plus the known payload sites: the
    // canonical stream above already covers all aligned decodes; this
    // exercises mid-entry (non-canonical) decode positions. Each offset's
    // decode depends only on the 6 bytes there (the tables are
    // position-free), so a windowed stdin feed is the whole ground truth.
    // One WSL bash loop handles all offsets (wsl.exe stdin piping from
    // Windows is not binary-safe; redirection inside WSL is).
    let plain_path = dir.join("TU102_sec_plain.bin");
    let plain = std::fs::read(&plain_path).expect("plaintext");
    let sites: [u64; 6] = [0x10b9, 0x1f89, 0xcbd, 0xc95, 0xe85, 0x7ea4];
    let mut offsets: Vec<usize> = (0..plain.len() - 6).step_by(97).collect();
    for s in sites {
        offsets.push((s - 0x100) as usize);
    }
    offsets.sort();
    offsets.dedup();
    let to_wsl = |p: &PathBuf| p.display().to_string().replace("\\", "/").replace("D:/", "/mnt/d/");
    let wsl_file = to_wsl(&plain_path);
    let mut script = String::new();
    for &off in &offsets {
        // envydis unconditionally prints an address line for positions
        // carrying a call-target mark (the switch(mark&3) path has no
        // quiet check), so each offset's chunk is delimited and the Rust
        // side takes the first non-address line within it.
        script.push_str(&format!(
            "tail -c +{} {wsl_file} | head -c 6 | {envy} -m falcon -V fuc5 -i -n -q; echo ===
",
            off + 1
        ));
    }
    let script_path = dir.join("falcgen_oracle_sample.sh");
    std::fs::write(&script_path, &script).expect("write script");
    let out = Command::new("wsl")
        .args(["-e", "bash", "-c", &format!("bash {}", to_wsl(&script_path))])
        .output()
        .expect("run wsl bash");
    assert!(out.status.success(), "oracle loop failed: {}", String::from_utf8_lossy(&out.stderr));
    let binding = String::from_utf8_lossy(&out.stdout);
    let is_addr_only = |l: &str| {
        let b = l.as_bytes();
        b.len() == 9 && b[..8].iter().all(|c| c.is_ascii_hexdigit()) && b[8] == b':'
    };
    let mut chunks = binding.split("===
").filter(|c| !c.is_empty());
    let mut mismatches = 0;
    for (i, &off) in offsets.iter().enumerate() {
        let chunk = chunks.next().unwrap_or_else(|| panic!("chunk {i} missing"));
        let line = chunk
            .lines()
            .find(|l| !l.is_empty() && !is_addr_only(l))
            .unwrap_or("???");
        let envy_mn = line.split_whitespace().next().unwrap_or("???").to_string();
        let ours = decode_one(&plain, off, 0x100, FUC5);
        let ours_mn = if ours.is_unknown() { "???".to_string() } else { ours.name.clone() };
        if envy_mn != ours_mn {
            mismatches += 1;
            if mismatches <= 10 {
                println!("offset {off:#x}: envydis {envy_mn:?} vs ours {ours_mn:?} ({})", ours.render());
            }
        }
    }
    assert!(mismatches == 0, "{mismatches}/{} sampled offsets disagree with envydis", offsets.len());
}

/// The variant gate itself: FUC5 must reject FUC0-only encodings while
/// accepting the fuc3+ aliases.
#[test]
fn variant_gating() {
    // fuc0-only movf (0x23d family, F_FUC0) resolves to movf under fuc0,
    // mov under fuc3+.
    let i = decode_one(&[0x3d, 0x02, 0x40, 0x00], 0, 0, Variant { fmask: 0x21, crypt: false });
    assert_eq!(i.name, "movf");
    let i = decode_one(&[0x3d, 0x02, 0x40, 0x00], 0, 0, FUC5);
    assert_eq!(i.name, "mov");
}
