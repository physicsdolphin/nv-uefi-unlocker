# diag feature split (raw strings preserve Rust source exactly).
# Feature model: tx = ["diag"] in Cargo.toml; shared surgery gates move to
# feature = "diag"; payload-only gates stay feature = "tx".
import io, re

def patch(path, subs):
    s = io.open(path, encoding='utf-8').read()
    for old, new in subs:
        n = s.count(old)
        assert n == 1, "count %d for %r in %s" % (n, old[:70], path)
        s = s.replace(old, new)
    io.open(path, 'w', encoding='utf-8', newline='').write(s)
    print("patched", path)

# 1. Cargo.toml (loader): tx implies diag; new diag feature.
patch(r'loader/Cargo.toml', [
    (
        "[features]\ndefault = []\n",
        "[features]\ndefault = []\n"
        "# Diagnostic profile (soft-lock family, #279 request): the FULL surgery\n"
        "# (detach/claim/COMMAND/DMA/bridge-BME/native-GOP rebuild/reconnect) plus\n"
        "# the observers (ebs-watch, agitate-only) with the payload paths (FWSEC\n"
        "# ignition, booter fire, selectors, transaction replay) EXCLUDED from the\n"
        "# binary - \\tx-agitate-only.txt is the only GPU-touching branch and it\n"
        "# writes nothing to BAR0 by contract. tx implies diag.\n"
        "diag = []\n"
    ),
    (
        "tx = []",
        "tx = [\"diag\"]",
    ),
])

# 2. main.rs module gates: shared surgery modules -> diag; payload-only stay tx.
patch(r'loader/src/main.rs', [
    (
        "#[cfg(feature = \"tx\")]\nmod tx;\n#[cfg(feature = \"tx\")]\nmod tx_policy;\n#[cfg(feature = \"tx\")]\nmod gop_native;\n#[cfg(feature = \"tx\")]\nmod gop_order;\n#[cfg(feature = \"tx\")]\nmod claim_plm;\n#[cfg(feature = \"tx\")]\nmod tx_fwsec;\n#[cfg(feature = \"tx\")]\nmod tx_booter;\n#[cfg(feature = \"tx\")]\nmod tx_detach;\n#[cfg(feature = \"tx\")]\nmod tx_portal;\n#[cfg(feature = \"tx\")]\nmod tx_prep;",
        "#[cfg(feature = \"diag\")]\nmod tx;\n#[cfg(feature = \"tx\")]\nmod tx_policy;\n#[cfg(feature = \"diag\")]\nmod gop_native;\n#[cfg(feature = \"diag\")]\nmod gop_order;\n#[cfg(feature = \"tx\")]\nmod claim_plm;\n#[cfg(feature = \"diag\")]\nmod tx_fwsec;\n#[cfg(feature = \"diag\")]\nmod tx_booter;\n#[cfg(feature = \"diag\")]\nmod tx_detach;\n#[cfg(feature = \"tx\")]\nmod tx_portal;\n#[cfg(feature = \"diag\")]\nmod tx_prep;",
    ),
    (
        "        #[cfg(feature = \"tx\")]\n        {\n            tx_prep::armed_confirm_with_prep(secs)\n        }\n        #[cfg(not(feature = \"tx\"))]\n        {\n            keywait::wait_confirm_armed(secs)\n        }",
        "        #[cfg(feature = \"diag\")]\n        {\n            tx_prep::armed_confirm_with_prep(secs)\n        }\n        #[cfg(not(feature = \"diag\"))]\n        {\n            keywait::wait_confirm_armed(secs)\n        }",
    ),
    (
        "    #[cfg(feature = \"tx\")]\n    let (gpus, prep_ran) = if let keywait::Confirm::Proceed = confirmed {\n        match tx_prep::take_gpus() {\n            Some(g) => (g, true),\n            None => (pci::enumerate(), false),\n        }\n    } else {\n        (pci::enumerate(), false)\n    };\n    #[cfg(not(feature = \"tx\"))]\n    let gpus = pci::enumerate();\n    #[cfg(not(feature = \"tx\"))]\n    let prep_ran = false;",
        "    #[cfg(feature = \"diag\")]\n    let (gpus, prep_ran) = if let keywait::Confirm::Proceed = confirmed {\n        match tx_prep::take_gpus() {\n            Some(g) => (g, true),\n            None => (pci::enumerate(), false),\n        }\n    } else {\n        (pci::enumerate(), false)\n    };\n    #[cfg(not(feature = \"diag\"))]\n    let gpus = pci::enumerate();\n    #[cfg(not(feature = \"diag\"))]\n    let prep_ran = false;",
    ),
    (
        "    #[cfg(feature = \"tx\")]\n    let hard_stop = if let keywait::Confirm::Proceed = confirmed {",
        "    #[cfg(feature = \"diag\")]\n    let hard_stop = if let keywait::Confirm::Proceed = confirmed {",
    ),
    (
        "    #[cfg(not(feature = \"tx\"))]\n    {\n        drop(confirmed);\n    }\n    #[cfg(not(feature = \"tx\"))]\n    let hard_stop = false;",
        "    #[cfg(not(feature = \"diag\"))]\n    {\n        drop(confirmed);\n    }\n    #[cfg(not(feature = \"diag\"))]\n    let hard_stop = false;",
    ),
    (
        "#[cfg(feature = \"tx\")]\nenum CardOutcome {",
        "#[cfg(feature = \"diag\")]\nenum CardOutcome {",
    ),
    (
        "#[cfg(feature = \"tx\")]\nfn tx_phase(gpus: &[pci::FoundGpu]) -> bool {",
        "#[cfg(feature = \"diag\")]\nfn tx_phase(gpus: &[pci::FoundGpu]) -> bool {",
    ),
    (
        "#[cfg(feature = \"tx\")]\nfn tx_one_card(g: &pci::FoundGpu) -> CardOutcome {",
        "#[cfg(feature = \"diag\")]\nfn tx_one_card(g: &pci::FoundGpu) -> CardOutcome {",
    ),
    (
        "#[cfg(feature = \"tx\")]\nfn asset_family(dev: u16) -> Option<&'static str> {",
        "#[cfg(feature = \"diag\")]\nfn asset_family(dev: u16) -> Option<&'static str> {",
    ),
])
print("phase 1 done")
