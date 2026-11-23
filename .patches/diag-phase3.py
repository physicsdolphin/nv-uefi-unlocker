# Phase 3: pci.rs infrastructure -> diag; payload fns -> tx-gated.
import io, re

def flip_all(path):
    s = io.open(path, encoding='utf-8').read()
    n = s.count('cfg(feature = "tx")')
    s = s.replace('cfg(feature = "tx")', 'cfg(feature = "diag")')
    io.open(path, 'w', encoding='utf-8', newline='').write(s)
    print(path, "flipped", n, "sites -> diag")

def gate_fns(path, names, attr='#[cfg(feature = "tx")]'):
    s = io.open(path, encoding='utf-8').read()
    for name in names:
        # top-level fn definition at column 0 (skip doc comments above - the
        # attribute lands between the docs and the fn, which is valid Rust).
        pat = re.compile(r'(?m)^(pub )?fn ' + re.escape(name) + r'\(')
        ms = list(pat.finditer(s))
        assert len(ms) == 1, (path, name, len(ms))
        m = ms[0]
        s = s[:m.start()] + attr + '\n' + s[m.start():]
    io.open(path, 'w', encoding='utf-8', newline='').write(s)
    print(path, "gated", len(names), "fns -> tx")

flip_all(r'loader/src/pci.rs')

gate_fns(r'loader/src/tx.rs', [
    'canary_arm',
    'falcon_reset',
    'falcon_dma_unblock',
    'authority_scan',
    'falcon_authority_probe',
    'claim_guards',
    'host_probe_reference',
    'ptimer_seed_reference',
    'pre_flr_experiment',
    'run_transaction',
    'pm_flr_pre',
    'pm_flr_post',
    'flr_once',
    'emergency_flr',
])

gate_fns(r'loader/src/tx_fwsec.rs', [
    'scrubber_if_needed',
    'fwsec_ignite',
    'fwsec_ignite_tu10x',
    'fwsec_aperture_dead_signature',
    'fuse_version',
])

gate_fns(r'loader/src/tx_booter.rs', [
    'boot_desc_parse',
    'load_booter',
    'booter_patch_stock',
    'wpr_meta_pack',
    'signature_restore',
    'booter_fire',
    'booter_fire_direct',
    'booter_fire_direct_c',
    'verify_execution',
    'verify_execution_tu10x',
    'write_selector',
    'flr_phase',
    'probe_post_console',
])
print("phase 3 done")
