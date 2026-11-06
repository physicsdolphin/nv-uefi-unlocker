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
        Some("dis") => cmd_dis(&args[1..]),
        Some("scan") => cmd_scan(&args[1..]),
        Some("lst-compare") => cmd_lst_compare(&args[1..]),
        Some("run") => cmd_run(&args[1..]),
        _ => usage(),
    }
    ExitCode::SUCCESS
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
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = parse_num(it.next().expect("--base value")),
            "--trace" => trace = true,
            "--steps" => steps = parse_num(it.next().expect("--steps value")),
            "--sig-len" => sig_len = parse_num(it.next().expect("--sig-len value")) as u32,
            "--sig-hex" => sig_hex = it.next().expect("--sig-hex value").to_string(),
            _ => file = Some(a.clone()),
        }
    }
    let buf = read_file(file.expect("input file"));
    let mut ip = falcgen::interp::Interp::new();
    ip.max_steps = steps;
    ip.trace = trace;
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
    }
}
