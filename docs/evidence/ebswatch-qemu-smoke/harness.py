# QEMU/OVMF smoke harness for the rc5 diagnostics (ebswatch + boundary lines).
# Rounds:
#   A selftest : \tx-ebs-selftest.txt (implies watch) -> full EBS sequence, parks
#   B boundary : bootmgfw.efi = a copy of the OVMF shell (valid PE) ->
#                LoadImage ok / StartImage enter boundary lines in loader.log
#   C negative : no markers -> no EBS-WATCH output at all
# Usage: python .patches/qemu-smoke.py <round>   (a|b|c)
import os, shutil, subprocess, sys, time, glob

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QEMU_DIR = os.path.join(REPO, 'sim', 'qemu')
RUN = os.path.join(QEMU_DIR, 'run')
USBROOT = os.path.join(QEMU_DIR, 'usbroot')
IMG = os.path.join(QEMU_DIR, 'usb.img')
LOADER = os.path.join(REPO, '.target-ebswatch', 'x86_64-unknown-uefi', 'release', 'loader.efi')
SHARE = r'C:\Program Files\QEMU\share'
# Copied out of Program Files: QEMU refuses -bios paths with spaces.
OVMF = os.path.join(QEMU_DIR, 'edk2-x86_64-code.fd')
QEMU = r'C:\Program Files\QEMU\qemu-system-x86_64.exe'

def wsl(cmd):
    # Windows paths inside /mnt/d/...: swap the repo prefix, then ALL
    # remaining backslashes (sh would otherwise eat '\s' as 's').
    wslrepo = REPO.replace('D:', '/mnt/d').replace('\\', '/')
    p = cmd.replace(REPO, wslrepo).replace('\\', '/')
    r = subprocess.run(['wsl', '-e', 'sh', '-c', p], capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stdout); print(r.stderr)
    return r

def build_image():
    # Fresh FAT32 from usbroot (64 MiB). mkfs.vfat cannot CREATE a file on
    # drvfs - preallocate with dd first.
    img_wsl = IMG.replace('D:', '/mnt/d').replace('\\', '/')
    usbroot_wsl = USBROOT.replace('D:', '/mnt/d').replace('\\', '/')
    r = wsl('dd if=/dev/zero of=%s bs=1M count=64 status=none && mkfs.vfat -F 32 -n NVRAMSTICK %s' % (img_wsl, img_wsl))
    assert r.returncode == 0, 'mkfs failed'
    r = wsl('cd %s && mcopy -s -n -i %s * ::/' % (usbroot_wsl, img_wsl))
    assert r.returncode == 0, 'mcopy failed: ' + r.stderr
    print('usb.img built from usbroot')

def stage(round_name):
    # Reset usbroot: fresh EFI/BOOT, no markers, no shell stub.
    shutil.rmtree(USBROOT, ignore_errors=True)
    os.makedirs(os.path.join(USBROOT, 'EFI', 'BOOT'))
    shutil.copy(LOADER, os.path.join(USBROOT, 'EFI', 'BOOT', 'BOOTX64.EFI'))
    with open(os.path.join(USBROOT, 'startup.nsh'), 'w', newline='\r\n') as f:
        f.write('FS0:\\EFI\\BOOT\\BOOTX64.EFI\r\n')
    if round_name == 'a':
        for name in ('tx-ebs-selftest.txt',):
            open(os.path.join(USBROOT, name), 'w').close()
    if round_name == 'b':
        # A VALID PE as the chain target: the OVMF bundle's own shell EFI.
        shells = glob.glob(os.path.join(SHARE, '**', '*Shell*.efi'), recursive=True) + \
                 glob.glob(os.path.join(SHARE, 'efi-shell', '*.efi'), recursive=True)
        # Fallback: edk2 does not ship the shell binary; use any PE from the
        # QEMU dir? None. Instead: run the shell built into OVMF by putting
        # startup.nsh path only - round B then still exercises the loader
        # chain? No: without a chain target, LoadImage never runs.
        # Practical fallback: use the ROUNDED-TRIP loader itself as the
        # chain target - it is a valid PE; StartImage re-enters it (banner,
        # 30s confirm, ABORT). Boundaries exercised both levels deep.
        os.makedirs(os.path.join(USBROOT, 'EFI', 'Microsoft', 'Boot'))
        shutil.copy(LOADER, os.path.join(USBROOT, 'EFI', 'Microsoft', 'Boot', 'bootmgfw.efi'))
        print('round B chain target = the loader itself (valid PE)')
    print('staged round', round_name)

def run_qemu(seconds, logname):
    serial_log = os.path.join(RUN, logname)
    args = [
        QEMU, '-machine', 'q35', '-m', '512', '-net', 'none',
        '-serial', 'stdio',
        # This QEMU build's -bios loader fails on every path; pflash works.
        '-drive', 'if=pflash,format=raw,readonly=on,file=' + OVMF,
        '-drive', 'if=none,id=usbstick,format=raw,file=' + IMG,
        '-device', 'qemu-xhci,id=xhci',
        '-device', 'usb-storage,bus=xhci.0,drive=usbstick',
    ]
    print('QEMU start (timeout %ss) -> %s' % (seconds, logname))
    with open(serial_log, 'wb') as out:
        p = subprocess.Popen(args, stdout=out, stderr=subprocess.STDOUT,
                             stdin=subprocess.DEVNULL, creationflags=subprocess.CREATE_NEW_PROCESS_GROUP)
        try:
            p.wait(timeout=seconds)
            print('QEMU exited on its own (rc=%s)' % p.returncode)
        except subprocess.TimeoutExpired:
            print('QEMU timeout - killing (expected for the selftest park)')
            p.kill()
            p.wait()
    return serial_log

def extract_logs():
    outdir = os.path.join(RUN, 'extract')
    shutil.rmtree(outdir, ignore_errors=True)
    os.makedirs(outdir)
    for name in ('loader.log', 'ebs-watch.log'):
        r = wsl('mcopy -i %s ::/%s %s 2>/dev/null; ls -la %s/%s' % (
            IMG, name,
            os.path.join(outdir, name).replace('D:', '/mnt/d').replace('\\', '/'),
            os.path.join(outdir).replace('D:', '/mnt/d').replace('\\', '/'), name))
    return outdir

if __name__ == '__main__':
    rnd = sys.argv[1] if len(sys.argv) > 1 else 'a'
    secs = int(sys.argv[2]) if len(sys.argv) > 2 else 120
    stage(rnd)
    build_image()
    log = run_qemu(secs, 'serial-rc5-%s.log' % rnd)
    extract = extract_logs()
    print('serial log:', log)
    print('extracted:', extract)
