#!/usr/bin/env python3
"""OVMF only; no PCI passthrough. Exit 33 = harness's isa-debug-exit success."""
import argparse
import pathlib
import shutil
import subprocess
import tempfile

p = argparse.ArgumentParser()
p.add_argument('ffs', type=pathlib.Path, help='read-only DXE .ffs from ffspack')
p.add_argument('--out', type=pathlib.Path, required=True)
p.add_argument('--code', default='/usr/share/edk2/x64/OVMF_CODE.4m.fd')
p.add_argument('--vars', default='/usr/share/edk2/x64/OVMF_VARS.4m.fd')
a = p.parse_args()
repo = pathlib.Path(__file__).resolve().parents[2]
subprocess.run(['cargo', 'build', '--release', '--manifest-path', str(repo/'tools/dxe-smoke/Cargo.toml'), '--target', 'x86_64-unknown-uefi'], check=True)
a.out.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='nv-dxe-smoke-') as tmp:
    tmp = pathlib.Path(tmp)
    root = tmp/'disk'
    (root/'EFI/BOOT').mkdir(parents=True)
    shutil.copy(repo/'tools/dxe-smoke/target/x86_64-unknown-uefi/release/dxe-smoke.efi', root/'EFI/BOOT/BOOTX64.EFI')
    shutil.copy(a.ffs, root/'driver.ffs')
    (root/'tx-dxe-arm.txt').write_text('1\n')
    for mode in ['armed', 'tamper', 'exhausted', 'interactive', 'headless', 'oversized-arm', 'malformed-counter']:
        (root/'mode.txt').write_text(mode+'\n')
        unattended = root/'tx-dxe-noconfirm.txt'
        if mode == 'interactive':
            unattended.unlink(missing_ok=True)
        else:
            unattended.touch()
        for name in ['loader.log', 'loader-prev.log']:
            (root/name).unlink(missing_ok=True)
        nv = tmp/'vars.fd'
        shutil.copy(a.vars, nv)
        cmd = ['qemu-system-x86_64', '-machine', 'q35', '-m', '512', '-display', 'none', '-serial', 'stdio', '-monitor', 'none', '-no-reboot',
               '-device', 'isa-debug-exit,iobase=0xf4,iosize=0x04',
               '-drive', f'if=pflash,format=raw,readonly=on,file={a.code}',
               '-drive', f'if=pflash,format=raw,file={nv}',
               '-drive', f'format=raw,file=fat:rw:{root}']
        result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=45)
        (a.out/f'{mode}.log').write_bytes(result.stdout)
        assert result.returncode == 33, (mode, result.returncode, result.stdout.decode(errors='replace'))
        assert f'SMOKE COMPLETE mode={mode}'.encode() in result.stdout
        log = (root/'loader.log').read_bytes() if (root/'loader.log').exists() else b''
        if log:
            (a.out/f'{mode}-loader.log').write_bytes(log)
        entered = b'FINAL: driver phases returned' in (result.stdout + log)
        assert entered == (mode in ['armed', 'headless']), (mode, 'wrong flow decision')
        print(f'{mode}: PASS (DXE dispatch + ReadyToBoot + budget + latch)')
