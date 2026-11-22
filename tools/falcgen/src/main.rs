use falcgen::{decode, isa, scan};
use std::collections::BTreeMap;
use std::process::ExitCode;

fn usage() -> ! {
    eprintln!(
        "usage: falcgen dis <file> [--base <va>] [--crypt] [--all-offsets]\n       falcgen scan <file> --base <va> [--crypt] [--check <va>,<va>,...]\n       falcgen lst-compare <file> <reference.lst> --base <va>\n       falcgen run <file> --base <va> [--trace] [--steps N] [--sig-len N] [--sig-hex XX..] [--break <pc>] [--no-stop-after-dma]\n       falcgen synth [--family tu102|tu11x|ga102|ad102] (--writes <addr>=<val>,... | --gate ss | --gate bar1 --bar1-size <GiB>) [--marker <hex>] [--golden <file>] [--out <file>]"
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

/// Synthesize the V67 payload for a write chain (raw, or a named gate
/// preset) and optionally compare byte-for-byte against a golden image:
///   falcgen synth --family tu11x --gate ss
///   falcgen synth --family ad102 --gate bar1 --bar1-size 64
///   falcgen synth --writes 0x409650=0xffffffff,0x1fa828=0 --golden <file>
/// Writes without '=' are treated as addr=0. The golden file and the
/// payload never enter git (the asset discipline). With --out, a manifest
/// sidecar (<out>.manifest.txt) records the chain, preconditions and any
/// UNPROVEN flags; execution on hardware is a separately gated decision.
fn cmd_synth(args: &[String]) {
    let mut writes_s: Option<String> = None;
    let mut gate: Option<String> = None;
    let mut bar1_size: Option<u32> = None;
    let mut marker_arg: Option<u32> = None;
    let mut golden: Option<String> = None;
    let mut out: Option<String> = None;
    let mut family = "tu102".to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--writes" => writes_s = Some(it.next().expect("--writes value").to_string()),
            "--gate" => gate = Some(it.next().expect("--gate value").to_string()),
            "--bar1-size" => {
                bar1_size = Some(parse_gib(it.next().expect("--bar1-size value")))
            }
            "--marker" => {
                marker_arg = Some(parse_num(it.next().expect("--marker value")) as u32)
            }
            "--golden" => golden = Some(it.next().expect("--golden value").to_string()),
            "--out" => out = Some(it.next().expect("--out value").to_string()),
            "--family" => family = it.next().expect("--family value").to_string(),
            other => {
                eprintln!("falcgen synth: unknown argument '{other}'");
                std::process::exit(2);
            }
        }
    }
    if !falcgen::synth::FAMILIES.contains(&family.as_str()) {
        eprintln!(
            "falcgen synth: unknown family '{family}' ({})",
            falcgen::synth::FAMILIES.join("|")
        );
        std::process::exit(2);
    }
    if gate.is_some() && writes_s.is_some() {
        eprintln!("falcgen synth: --gate and --writes are mutually exclusive");
        std::process::exit(2);
    }

    // Resolve the write chain: a named gate preset or the raw --writes list.
    // `proven` tracks the evidence level reported in the manifest: presets
    // reproduce pinned tables; a raw chain is novel until hardware-proven.
    let mut proven = gate.is_some();
    let gate_desc: String;
    let writes: Vec<(u32, u32)> = match gate.as_deref() {
        Some("ss") => {
            gate_desc = "ss (stock per-family unlock chain)".to_string();
            falcgen::synth::preset_ss(&family).expect("ss preset exists for every family")
        }
        Some("bar1") => {
            if family != "ad102" {
                eprintln!("falcgen synth: the bar1 gate is AD102-only (the CYA/CFG SEC2 construction has no other-family evidence)");
                std::process::exit(2);
            }
            let gib = bar1_size.unwrap_or_else(|| {
                eprintln!("falcgen synth: --gate bar1 requires --bar1-size <GiB> (2^n, 16..1024)");
                std::process::exit(2);
            });
            let (w, is_proven) = falcgen::synth::preset_bar1(gib).unwrap_or_else(|| {
                eprintln!(
                    "falcgen synth: --bar1-size {gib} is not a 2^n GiB size in 16..1024 (the 64MiB<<i size-table domain)"
                );
                std::process::exit(2);
            });
            proven = is_proven;
            gate_desc = format!(
                "bar1 size {gib} GiB ({})",
                if is_proven {
                    "PROVEN — byte-verified against the gpu-unlock-bar1 eight-card artifact"
                } else {
                    "UNPROVEN — 2^n-table construction, simulation-validated only"
                }
            );
            w
        }
        Some(other) => {
            eprintln!("falcgen synth: unknown gate '{other}' (ss|bar1)");
            std::process::exit(2);
        }
        None => {
            gate_desc = "custom write chain (operands unproven on hardware)".to_string();
            proven = false;
            writes_s
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|w| {
                    let (a, v) = w.split_once('=').unwrap_or((w, "0"));
                    (
                        u32::from_str_radix(a.trim().trim_start_matches("0x"), 16).expect("addr"),
                        u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).unwrap_or(0),
                    )
                })
                .collect()
        }
    };

    // Marker: only the AD102 payload carries a mailbox marker; the other
    // families' verdicts are PLM-flip / readback-only. The bar1 gate
    // defaults to its own marker (0x67b00004, the eight-card artifact).
    let marker = match (marker_arg, family.as_str()) {
        (Some(m), "ad102") => m,
        (Some(_), _) => {
            eprintln!("falcgen synth: --marker only applies to ad102 (the only payload with a mailbox marker)");
            std::process::exit(2);
        }
        (None, "ad102") if gate.as_deref() == Some("bar1") => falcgen::synth::MARKER_AD102_BAR1,
        (None, "ad102") => falcgen::synth::MARKER_AD102,
        (None, _) => 0,
    };

    if writes.is_empty() {
        eprintln!("falcgen synth: the write chain must not be empty");
        std::process::exit(2);
    }
    let budget = falcgen::synth::write_budget(&family);
    if writes.len() > budget {
        eprintln!(
            "falcgen synth: {} writes exceed the {family} budget of {budget}\nper-family frame budget (chain frames between chain_base and tail_base / put-list capacity):\n{}\na larger chain needs frame relocation (pivot, cmp50hx stockflow style) — see docs/gate-inventory.md",
            writes.len(),
            falcgen::synth::budget_table()
        );
        std::process::exit(2);
    }

    let geo = falcgen::synth::Geometry::tu102();
    let payload = match family.as_str() {
        "tu102" => falcgen::synth::synth(&writes, &geo),
        "tu11x" => falcgen::synth::synth(&writes, &falcgen::synth::Geometry::tu11x()),
        "ga102" => falcgen::synth::synth_ga102(&writes),
        "ad102" => falcgen::synth::synth_ad102(&writes, marker),
        _ => unreachable!("family validated above"),
    };
    let got = falcgen::synth::sha256_hex(&payload);
    println!("payload: {} bytes, sha256 {}", payload.len(), got);
    if let Some(path) = out {
        std::fs::write(&path, &payload).expect("write payload");
        let manifest = manifest_text(&family, &gate_desc, &writes, marker, proven, &got);
        let manifest_path = format!("{path}.manifest.txt");
        std::fs::write(&manifest_path, &manifest).expect("write manifest");
        println!("manifest: {manifest_path}");
    }
    if let Some(path) = golden {
        let want = std::fs::read(&path).expect("read golden");
        let want_hex = falcgen::synth::sha256_hex(&want);
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

fn parse_gib(s: &str) -> u32 {
    let t = s
        .trim()
        .trim_end_matches("iB")
        .trim_end_matches("ib")
        .trim_end_matches(['G', 'g']);
    parse_num(t) as u32
}

/// The manifest sidecar: write chain, verdict preconditions and evidence
/// level. English only (the runtime-output convention).
fn manifest_text(
    family: &str,
    gate_desc: &str,
    writes: &[(u32, u32)],
    marker: u32,
    proven: bool,
    sha: &str,
) -> String {
    let wl = writes
        .iter()
        .map(|(a, v)| format!("{a:#x}={v:#x}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut m = String::new();
    m.push_str("payload manifest (offline artifact; hardware execution is a separately gated decision)\n");
    m.push_str(&format!("  family:  {family}\n"));
    m.push_str(&format!("  gate:    {gate_desc}\n"));
    m.push_str(&format!("  writes:  {wl}\n"));
    if family == "ad102" {
        m.push_str(&format!("  marker:  {marker:#x}\n"));
    }
    m.push_str(&format!("  sha256:  {sha}\n"));
    m.push_str("  preconditions:\n");
    m.push_str("    - cold FEAT PLM write-enable nibble [7:4] must read 8 (0 = professional template: the payload write cannot latch, issue #237)\n");
    match (family, gate_desc.starts_with("bar1")) {
        ("tu102", _) | ("tu11x", _) => {
            m.push_str("    - Turing handoff: GSP-RM rejects the unlock state on TU11x in BOTH shapes (open #246; relocked 0xffffffef #280) - the relock tail is proven on TU102 only (#103/#240); the TU11x legacy-driver lane and a full-factory 0xffffff8f handoff are the open discriminators\n");
        }
        ("ga102", _) => {
            m.push_str("    - GA10x verdict is the wait-PLM flip; SS0=0x88888888 / SS1 low nibble 8 are HOST-side writes after the flip\n");
        }
        ("ad102", true) => {
            m.push_str("    - AD102 BAR1: host CYA/CFG writes are dropped even with the PLMs open; the SEC2 payload writes are the only proven path (eight-card evidence)\n");
            m.push_str("    - verify post-FLR: the four target readbacks plus ReBAR-capability consistency\n");
        }
        ("ad102", false) => {
            m.push_str("    - Ada verdict is the marker/HALT/readback triad; FBPA reading 0xffffff0f is non-fatal\n");
        }
        _ => {}
    }
    if !proven {
        m.push_str("    - UNPROVEN: simulation-validated only; no hardware evidence exists for this construction\n");
    }
    m
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
    let mut no_stop_after_dma = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = parse_num(it.next().expect("--base value")),
            "--trace" => trace = true,
            "--steps" => steps = parse_num(it.next().expect("--steps value")),
            "--sig-len" => sig_len = parse_num(it.next().expect("--sig-len value")) as u32,
            "--sig-hex" => sig_hex = it.next().expect("--sig-hex value").to_string(),
            "--break" => brk = Some(parse_num(it.next().expect("--break value")) as u32),
            "--no-stop-after-dma" => no_stop_after_dma = true,
            _ => file = Some(a.clone()),
        }
    }
    let buf = read_file(file.expect("input file"));
    let mut ip = falcgen::interp::Interp::new();
    ip.max_steps = steps;
    ip.trace = trace;
    ip.break_pc = brk;
    ip.stop_after_dma = !no_stop_after_dma;
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
        println!("io writes: {} total; last 40: {:?}", n,
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
