use falcgen::{decode, isa, scan};
use std::collections::BTreeMap;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!(
        "usage: falcgen dis <file> [--base <va>] [--crypt] [--all-offsets]\n       falcgen scan <file> --base <va> [--crypt] [--check <va>,<va>,...]\n       falcgen lst-compare <file> <reference.lst> --base <va>
       falcgen run <file> --base <va> [--trace] [--steps N] [--sig-len N] [--sig-hex XX..]"
    );
    std::process::exit(2);
}

fn parse_num(s: &str) -> u64 {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).expect("hex number")
    } else {
        s.parse().expect("number")
    }
}

fn read_file(path: String) -> Vec<u8> {
    std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("falcgen: cannot read {path}: {e}");
        std::process::exit(1);
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("synth") => cmd_synth(&args[1..]),
        Some("dis") => cmd_dis(&args[1..]),
        Some("scan") => cmd_scan(&args[1..]),
        Some("lst-compare") => cmd_lst_compare(&args[1..]),
        Some("run") => cmd_run(&args[1..]),
        _ => usage(),
    }
    ExitCode::SUCCESS
}

/// Synthesize the V67 payload for a write chain and optionally compare
/// byte-for-byte against a golden image:
///   falcgen synth --writes 0x409650=0xffffffff,0x1fa828=0 --golden <file>
/// Writes without '=' are treated as addr=0. The golden file never enters
/// git (the payload stays out of the repo per the asset discipline).
fn cmd_synth(args: &[String]) {
    let mut writes_s = String::new();
    let mut golden: Option<String> = None;
    let mut out: Option<String> = None;
    let mut family = "tu102".to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--writes" => writes_s = it.next().expect("--writes value").to_string(),
            "--golden" => golden = Some(it.next().expect("--golden value").to_string()),
            "--out" => out = Some(it.next().expect("--out value").to_string()),
            "--family" => family = it.next().expect("--family value").to_string(),
            _ => {}
        }
    }
    let mut writes: Vec<(u32, u32)> = Vec::new();
    for w in writes_s.split(',').filter(|s| !s.is_empty()) {
        let (a, v) = w.split_once('=').unwrap_or((w, "0"));
        writes.push((
            u32::from_str_radix(a.trim().trim_start_matches("0x"), 16).expect("addr"),
            u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).unwrap_or(0),
        ));
    }
    let geo = falcgen::synth::Geometry::tu102();
    let payload = match family.as_str() {
        "tu102" => falcgen::synth::synth(&writes, &geo),
        "tu11x" => falcgen::synth::synth(&writes, &falcgen::synth::Geometry::tu11x()),
        "ga102" => falcgen::synth::synth_ga102(&writes),
        "ad102" => falcgen::synth::synth_ad102(&writes),
        other => {
            eprintln!("falcgen synth: unknown family '{other}' (tu102|ga102|ad102)");
            std::process::exit(2);
        }
    };
    let got = sha256_hex(&payload);
    println!("payload: {} bytes, sha256 {}", payload.len(), got);
    if let Some(path) = out {
        std::fs::write(path, &payload).expect("write payload");
    }
    if let Some(path) = golden {
        let want = std::fs::read(&path).expect("read golden");
        let want_hex = sha256_hex(&want);
        println!("golden:  {} bytes, sha256 {}", want.len(), want_hex);
        if payload.len() == want.len() && payload == want {
            println!("BYTE-IDENTICAL ✓");
        } else {
            let mut diffs = 0usize;
            for i in 0..payload.len().min(want.len()) {
                if payload[i] != want[i] {
                    diffs += 1;
                    if diffs <= 8 {
                        println!("  diff @ {i:#x} (dmem {:#x}): ours {:02x} want {:02x}",
                            i as u32 + geo.copy_base, payload[i], want[i]);
                    }
                }
            }
            println!("{diffs} differing bytes (showing first 8)");
            std::process::exit(1);
        }
    }
}

fn sha256_hex(data: &[u8]) -> String {
    // minimal SHA-256 (no external deps in this crate)
    let mut h = [0u32; 8];
    h.copy_from_slice(&[
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ]);
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (hi, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hi = hi.wrapping_add(v);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

fn variant(crypt: bool) -> isa::Variant {
    if crypt {
        isa::FUC5_CRYPT
    } else {
        isa::FUC5
    }
}

fn cmd_dis(args: &[String]) {
    let mut file = None;
    let mut base = 0u64;
    let mut crypt = false;
    let mut all = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = parse_num(it.next().expect("--base value")),
            "--crypt" => crypt = true,
            "--all-offsets" => all = true,
            _ => file = Some(a.clone()),
        }
    }
    let buf = read_file(file.expect("input file"));
    let var = variant(crypt);
    if all {
        for off in 0..buf.len() {
            let insn = decode::decode_one(&buf, off, base, var);
            println!("{off:06x} {}: {}", insn.oplen, insn.render());
        }
    } else {
        for insn in decode::decode_linear(&buf, base, var) {
            if insn.is_unknown() {
                println!("{:06x} n={} {} [unknown: {}]", insn.pos, insn.oplen, insn.render(),
                    insn.unknown.iter().map(|b| format!("{b:02x}")).collect::<String>());
            } else {
                println!("{:06x} n={} {}", insn.pos, insn.oplen, insn.render());
            }
        }
    }
}

fn cmd_scan(args: &[String]) {
    let mut file = None;
    let mut base = None;
    let mut crypt = false;
    let mut check: Vec<u64> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = Some(parse_num(it.next().expect("--base value"))),
            "--crypt" => crypt = true,
            "--check" => {
                for s in it.next().expect("--check list").split(',') {
                    check.push(parse_num(s));
                }
            }
            _ => file = Some(a.clone()),
        }
    }
    let buf = read_file(file.expect("input file"));
    let base = base.expect("--base required");
    let map: BTreeMap<u64, scan::Candidate> = scan::scan(&buf, base, variant(crypt));
    println!("candidates: {}", map.len());
    if !check.is_empty() {
        for (va, found) in scan::check_sites(&map, &check) {
            match found {
                Some(tags) => println!("  {va:#07x}: HIT {tags:?}"),
                None => println!("  {va:#07x}: miss"),
            }
        }
    }
    for (va, c) in map.iter().take(40) {
        println!("  {va:#07x} n={} {:?} {}", c.len, c.tags, c.text);
    }
}

/// Compare our linear decode against an envydis lst (offset, length,
/// mnemonic only). The reference lst addresses file offsets directly; our
/// base is added to positions, so pass --base 0 for lst comparison.
fn cmd_lst_compare(args: &[String]) {
    let mut file = None;
    let mut lst = None;
    let mut base = 0u64;
    let mut crypt = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = parse_num(it.next().expect("--base value")),
            "--crypt" => crypt = true,
            _ => {
                if file.is_none() {
                    file = Some(a.clone());
                } else if lst.is_none() {
                    lst = Some(a.clone());
                }
            }
        }
    }
    let buf = read_file(file.expect("input file"));
    let lst_text = std::fs::read_to_string(lst.expect("reference lst")).expect("lst");
    // reference: (offset, mnemonic) pairs from the lst's canonical stream
    let want = decode::parse_lst(&lst_text);
    let var = variant(crypt);
    let got = decode::decode_linear(&buf, base, var);
    let mut mismatches = 0usize;
    for (i, (off, mn)) in want.iter().enumerate() {
        match got.get(i) {
            // lst addresses are file offsets of the plaintext; our pos is
            // base + file offset, so compare modulo the base.
            Some(insn) if insn.pos - base == *off && insn.name == *mn => {}
            Some(insn) => {
                mismatches += 1;
                if mismatches <= 10 {
                    println!("MISMATCH #{i}: lst {off:06x} {mn} vs got {:06x} {} ({})", insn.pos - base, insn.name, insn.render());
                }
            }
            None => {
                mismatches += 1;
                if mismatches <= 10 {
                    println!("MISMATCH #{i}: lst {off:06x} {mn} vs got <end>");
                }
            }
        }
    }
    println!("reference {} instructions, ours {}, mismatches {}", want.len(), got.len(), mismatches);
    if mismatches > 0 || want.len() != got.len() {
        std::process::exit(1);
    }
}

fn cmd_run(args: &[String]) {
    let mut file = None;
    let mut base = 0u64;
    let mut trace = false;
    let mut steps = 20_000_000u64;
    let mut sig_len = 0x10u32;
    let mut sig_hex = String::new();
    let mut brk: Option<u32> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = parse_num(it.next().expect("--base value")),
            "--trace" => trace = true,
            "--steps" => steps = parse_num(it.next().expect("--steps value")),
            "--sig-len" => sig_len = parse_num(it.next().expect("--sig-len value")) as u32,
            "--sig-hex" => sig_hex = it.next().expect("--sig-hex value").to_string(),
            "--break" => brk = Some(parse_num(it.next().expect("--break value")) as u32),
            _ => file = Some(a.clone()),
        }
    }
    let buf = read_file(file.expect("input file"));
    let mut ip = falcgen::interp::Interp::new();
    ip.max_steps = steps;
    ip.trace = trace;
    ip.break_pc = brk;
    ip.seed_meta_sig_len(sig_len);
    if !sig_hex.is_empty() {
        let bytes: Vec<u8> = (0..sig_hex.len() / 2)
            .map(|i| u8::from_str_radix(&sig_hex[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        ip.host_buffers.push(bytes);
    } else {
        ip.host_buffers.push(vec![0xaau8; 0x1000]);
    }
    ip.run(&buf, base);
    println!("halted={} reason={} steps={} last_dma_sp={:?}", ip.halted, ip.exit_reason, ip.steps, ip.last_dma_sp);
    println!("dma copies: {:?}", ip.dma_copies.iter().map(|d| (d.dst, d.len)).collect::<Vec<_>>());
    if let Some(sp) = ip.last_dma_sp {
        println!("stack dump (sp={sp:#x}):");
        let top = (sp as usize).min(0x10000);
        let lo = top.saturating_sub(0x100);
        let mut a = lo;
        while a + 4 <= 0x10000 {
            let v = u32::from_le_bytes(ip.dmem[a..a + 4].try_into().unwrap());
            let tag = if v == 0xd2408 { " <== canary" } else if (0x100..0x8500).contains(&v) { " <== imem-range (ret?)" } else { "" };
            println!("  {a:#06x}: {v:#010x}{tag}");
            a += 4;
        }
    } else {
        let n = ip.io_writes.len();
        println!("io writes: {} total; last 12: {:?}", n,
            ip.io_writes.iter().rev().take(40).rev().map(|(a, v)| (format!("{a:#x}"), format!("{v:#x}"))).collect::<Vec<_>>());
        // stack-tail dump (the validator frames live at 0xff00+)
        let mut a = 0xff00usize;
        while a + 4 <= 0x10000 {
            let v = u32::from_le_bytes(ip.dmem[a..a + 4].try_into().unwrap());
            if v != 0 {
                println!("  {a:#06x}: {v:#010x}");
            }
            a += 4;
        }
    }
}
