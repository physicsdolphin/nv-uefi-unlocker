# tx_booter gating with the real function names (load_booter takes a bundle
# slice, not booter; flr_phase/verify live at the tail).
import io, re

def gate_fns(path, names, attr='#[cfg(feature = "tx")]'):
    s = io.open(path, encoding='utf-8').read()
    for name in names:
        pat = re.compile(r'(?m)^(pub )?fn ' + re.escape(name) + r'(?:<[^>(]*>)?\(')
        ms = list(pat.finditer(s))
        assert len(ms) == 1, (path, name, len(ms))
        m = ms[0]
        s = s[:m.start()] + attr + '\n' + s[m.start():]
    io.open(path, 'w', encoding='utf-8', newline='').write(s)
    print(path, "gated", len(names), "fns -> tx")

gate_fns(r'loader/src/tx_booter.rs', [
    'boot_desc_parse',
    'load_booter',
    'booter_patch_stock',
    'booter_patch_v67',
    'wpr_meta_pack',
    'signature_restore',
    'write_selector',
    'booter_fire',
    'booter_fire_direct',
    'booter_fire_direct_c',
    'verify_execution',
    'verify_execution_tu10x',
    'flr_phase',
    'probe_post_console',
])
print("done")
