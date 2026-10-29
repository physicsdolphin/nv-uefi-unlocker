//! TU102 1e07/22GiB的隔离修复候选。保留单次点火，不复制C的失败重试。
use alloc::{format, vec, vec::Vec};
use core::time::Duration;
use log::info;
use sha2::{Digest, Sha256};
use crate::{pci::DmaBuffer, tx::{TxAssets, TxError, TxGpu}, tu102_profile as p};
use p::Mmio;

struct Live<'a, 'b>(&'a TxGpu<'b>);
impl Mmio for Live<'_, '_> {
    fn read(&mut self, r: u32) -> u32 { self.0.read32_raw(r) }
    fn write(&mut self, r: u32, v: u32) { self.0.write32(r, v); }
    fn stall(&mut self, us: u64) { uefi::boot::stall(Duration::from_micros(us)); }
    fn flush(&mut self) { unsafe { core::arch::asm!("wbinvd", options(nostack, preserves_flags)); } }
}
fn hash(bytes: &[u8]) -> alloc::string::String { format!("{:x}", Sha256::digest(bytes)) }
fn checked(data: &[u8], expected: &str) -> Result<(), TxError> {
    if hash(data) != expected { return Err(TxError::EInval) } Ok(())
}

pub fn preflight(gpu: &TxGpu, a: &TxAssets, device: u16) -> Result<(), TxError> {
    if let Some(cap) = gpu.pci.pm_cap() {
        if gpu.pci.pmcsr(cap)&3!=0 { return Err(TxError::EWarm) }
    }
    if gpu.read32(0x1fa828)? != 0 { return Err(TxError::EBusy) }
    if a.gsp_image.len()!=p::IMAGE_SIZE || a.gsp_image.iter().any(|v|*v!=0) {
        return Err(TxError::EInval);
    }
    let mut words = vec![0u32; 0x4000];
    if gpu.pci.bar_read_u32_buf(0, 0x300000, &mut words) != uefi::Status::SUCCESS { return Err(TxError::EIO) }
    let prom = unsafe { core::slice::from_raw_parts(words.as_ptr().cast::<u8>(), 0x10000) };
    let crc = uefi::boot::calculate_crc32(prom).map_err(|_| TxError::EIO)?;
    if a.family != "tu10x" || !p::identity(device, gpu.read32(0)?, crc, gpu.read32(0x100ce0)?) {
        return Err(TxError::ENodev);
    }
    for marker in ["\\pre-flr-on.txt", "\\tx-force-continue.txt", "\\tx-gop-disconnect.txt",
        "\\tx-low-staging.txt", "\\tx-meta-in-vram.txt", "\\i_know_what_i_am_doing.txt"] {
        if crate::tx::read_boot_file(marker).is_some() { return Err(TxError::EInval) }
    }
    let (_, image, sig) = crate::tx_booter::load_booter(&a.booter, "tu10x")?;
    checked(image, "e0f0fc93da097b5b68e5af3adb58b2235ed1fad5fce0b23592d3895af44cb743")?;
    if sig != [0x32,0x60,0x43,0x67,0x98,0x89,0x7b,0x06,0xe8,0x5f,0x1a,0xa8,0x71,0x43,0x5d,0xcb] {
        return Err(TxError::EInval);
    }
    checked(a.v67.as_ref().ok_or(TxError::EInval)?, p::PAYLOAD_SHA)?;
    checked(&a.gsp_boot, "de676bae28a480011d3d012db14bef539324e62a841a9627863c689bea168af3")?;
    checked(&a.stock_sig,"f5020943a001134927f247f5d9aeccb38a51111fcd490bc91eade76c6e1f437e")?;
    checked(a.bl.as_ref().ok_or(TxError::EInval)?, "f21f1cfbb8fffa8067685314367fc4c50f3f84dd99eda7c19db4292d9269b9d9")?;
    // 固件身份也在任何GPU写操作之前验证，避免误把其他VBIOS当成本机。
    let native = native_fwsec(gpu)?;
    checked(&native, p::FWSEC_SHA)?;
    info!("TU102-C: exact device/PROM/capacity/firmware/payload identity verified");
    Ok(())
}

fn native_fwsec(gpu: &TxGpu) -> Result<Vec<u8>, TxError> {
    let mut rom = vec![0u32; 1024*1024/4];
    if gpu.pci.bar_read_u32_buf(0, 0x300000, &mut rom) != uefi::Status::SUCCESS { return Err(TxError::EIO) }
    let bytes = unsafe { core::slice::from_raw_parts(rom.as_ptr().cast::<u8>(), 1024*1024) };
    let f = crate::tx_fwsec::prom_find_fwsec(bytes, false).ok_or(TxError::EInval)?;
    if (f.imem_size, f.imem_sec_base, f.imem_sec_size, f.data_off, f.dmem_size, f.interface_off, f.stored_size)
        != (0x9600,0x300,0x9300,0x9600,0x410,0xdc,0x9a10) { return Err(TxError::EInval) }
    Ok(bytes[f.image_off as usize..(f.image_off+f.stored_size) as usize].to_vec())
}
fn allocate(gpu: &TxGpu, buffers: &mut Vec<DmaBuffer>, n: usize) -> Result<(*mut u8,u64), TxError> {
    let b = gpu.pci.dma_alloc_tu102_compat(n).map_err(|_| TxError::ENodev)?;
    let result = (b.host,b.dev);
    buffers.push(b);
    Ok(result)
}
fn copy(ptr: *mut u8, data: &[u8]) {
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()); }
}
fn fwsec(gpu: &TxGpu, a: &TxAssets, buffers: &mut Vec<DmaBuffer>) -> Result<(), TxError> {
    let mut io = Live(gpu);
    let native = native_fwsec(gpu)?;
    checked(&native, p::FWSEC_SHA)?;
    let (host, dev) = allocate(gpu, buffers, 0x9b00)?;
    copy(host, &native);
    let image = unsafe { core::slice::from_raw_parts_mut(host, 0x9b00) };
    // 继续使用本机PROM解析器修补FRTS，绝不嵌入别人的FWSEC。
    let mut rom = vec![0u32; 1024*1024/4];
    if gpu.pci.bar_read_u32_buf(0,0x300000,&mut rom) != uefi::Status::SUCCESS { return Err(TxError::EIO) }
    let bytes = unsafe { core::slice::from_raw_parts(rom.as_ptr().cast::<u8>(),1024*1024) };
    let f = crate::tx_fwsec::prom_find_fwsec(bytes,false).ok_or(TxError::EInval)?;
    crate::tx_fwsec::fwsec_patch_frts(image,&f,p::FRTS)?;
    const GSP: u32 = 0x110000;
    io.write(GSP+0x80,0);
    for _ in 0..10_000 {
        if io.read(GSP+0xf4)&0x80000000!=0 { break }
        io.stall(100);
    }
    p::pulse(&mut io,GSP);
    p::wait_scrub(&mut io,GSP,30_000).map_err(|_|TxError::EIO)?;
    io.stall(50_000);
    io.write(0x111668,0);
    for _ in 0..16 { io.read(0x111668); }
    io.write(GSP+0x84,0x162000a1);
    for _ in 0..16 { io.read(GSP+0x84); }
    let ctl = gpu.read32(GSP+0x624)?;
    io.write(GSP+0x624,ctl|0x80);
    io.write(GSP+0x10c,0);
    io.write(GSP+0x600,0x15);
    io.write(GSP+0x610,0x15);
    let desc = p::bl_desc(dev).ok_or(TxError::ERange)?;
    p::load_dmem(&mut io,GSP,&desc);
    io.flush();
    io.write(GSP+0x1c0,0);
    let blocks = gpu.read32(GSP+0x108)? & 0x1ff;
    if blocks < 2 { return Err(TxError::EInval) }
    let dst = (blocks-2)<<8;
    let bl = &a.bl.as_ref().ok_or(TxError::EInval)?[..0x200];
    p::load_imem(&mut io,GSP,dst,0xfd00,bl,false);
    io.flush();
    io.write(GSP+0x180,dst);
    if io.read(GSP+0x184) != u32::from_le_bytes(bl[..4].try_into().unwrap()) { return Err(TxError::EIO) }
    io.write(GSP+0x1c0,0x20);
    if io.read(GSP+0x1c4) != 4 { return Err(TxError::EIO) }
    io.write(GSP+0x1c0,0);
    io.write(GSP+0x104,0xfd00);
    io.flush();
    io.write(GSP+0x100,2);
    let mut up = false;
    for _ in 0..5000 {
        if io.read(0x1fa824)==0x57fe000 && io.read(0x1fa828)==0x57fee00 { up=true; break }
        io.stall(1000);
    }
    if !up { return Err(TxError::EIO) }
    // WPR2出现不能充当完成信号，杜绝GSP仍执行时提前点火SEC2或复用固件内存。
    p::wait_halt(&mut io,GSP,50_000).map_err(|_| TxError::EIO)?;
    info!("TU102-C: FWSEC exact WPR2 and GSP HALT verified; buffer retained until cleanup");
    Ok(())
}

pub fn run(gpu: &TxGpu, a: &TxAssets, buffers: &mut Vec<DmaBuffer>) -> Result<(), TxError> {
    if p::unlocked(gpu.read32_raw(p::SS0),gpu.read32_raw(p::SS1)) { return Err(TxError::EAlready) }
    let mut io = Live(gpu);
    let cmd = gpu.pci.config_read_u16(4);
    gpu.pci.cfg_write_u16(4,cmd|6);
    if gpu.pci.config_read_u16(4)&6!=6 { return Err(TxError::EIO) }
    if p::host_probe(&mut io) { return Ok(()) }
    crate::tx::gfw_wait(gpu)?;
    let seed = uefi::runtime::get_time().ok().and_then(|t|
        p::reference_timer_ns(t.year(),t.month(),t.hour(),t.minute(),t.second()))
        .unwrap_or(1787011200000000000);
    io.write(0x9410,(seed>>32) as u32); io.write(0x9400,seed as u32);
    info!("TU102-C: reference PTIMER seed={seed:#x}");
    // 分配顺序与成功C首轮一致：载荷、Booter、GSP BL、dummy、radix、meta、FWSEC。
    let (sig, sig_dev)=allocate(gpu,buffers,0xfa00)?;
    copy(sig,a.v67.as_ref().ok_or(TxError::EInval)?);
    let (hdr, raw, signatures)=crate::tx_booter::load_booter(&a.booter,"tu10x")?;
    let (booter, _)=allocate(gpu,buffers,raw.len())?;
    copy(booter,raw);
    let image=unsafe { core::slice::from_raw_parts_mut(booter,raw.len()) };
    crate::tx_booter::booter_patch_stock(image,&hdr,signatures,0)?;
    let (boot,boot_dev)=allocate(gpu,buffers,0x6000)?; copy(boot,&a.gsp_boot);
    let (stock,stock_dev)=allocate(gpu,buffers,p::IMAGE_SIZE)?;
    // C的原始dummy拷贝在radix发布后已无消费者，复用其开头保留原签名恢复副本。
    // radix仍来自全零独立镜像，不受这份签名副本影响。
    copy(stock,&a.stock_sig);
    let radix=crate::tx_booter::radix_build_with(&a.gsp_image,|n|
        gpu.pci.dma_alloc_tu102_compat(n).map_err(|_|TxError::ENodev))?;
    let radix_dev=radix.level0; buffers.push(radix.dma);
    let (meta,meta_dev)=allocate(gpu,buffers,256)?;
    copy(meta,&p::meta(radix_dev,sig_dev,boot_dev).ok_or(TxError::EInval)?);
    io.flush();
    p::pulse(&mut io,0x110000); io.stall(200_000); // POST GFW退出，再执行本机FWSEC
    fwsec(gpu,a,buffers)?;
    io.write(0x1fa824,0x1ffe000); io.write(0x1fa828,0x1ffee00);
    io.flush(); io.stall(5000);
    if io.read(0x1fa824)!=0x57fe000 || io.read(0x1fa828)!=0x57fee00 { return Err(TxError::EIO) }
    info!("TU102-C: before booter GSP={:#x} SEC2={:#x} RESET_PLM={:#x} meta={meta_dev:#x}",
        io.read(0x110100),io.read(0x840100),io.read(0x8403c4));
    let result=p::fire_direct(&mut io,image,meta_dev);
    info!("TU102-C: booter {:?} mailbox0={:#x} PLM={:#x} SS0={:#x} SS1={:#x}",result,
        io.read(0x840040),io.read(p::PLM),io.read(p::SS0),io.read(p::SS1));
    // 在释放DMA前撤销宿主meta中的超长签名引用；失败也复原，不进行二次点火。
    crate::tx_booter::signature_restore(unsafe { core::slice::from_raw_parts_mut(meta,256) },stock_dev,a.stock_sig.len() as u64);
    io.flush();
    result.map_err(|_|TxError::EIO)?;
    p::sanitize(&mut io).map_err(|_|TxError::EIO)?;
    info!("TU102-C: post-sanitize SS0/SS1 verified; no FLR or GOP reconnect");
    Ok(())
}

pub fn release_after_handoff(gpu: &TxGpu, buffers: &mut Vec<DmaBuffer>, command: u16) {
    // 主调方失败FLR之后或成功sanitize之后才调用，绝不在固件仍执行时free。
    let halted=|r| { let v=gpu.read32_raw(r); v!=u32::MAX && v&0xffff0000!=0xbadf0000 && v&0x10!=0 };
    if !buffers.is_empty() && (!halted(0x110100)||!halted(0x840100)) {
        info!("TU102-C: engines not both halted; retaining {} reserved buffers until reboot",buffers.len());
    } else {
        let mut all_released=true;
        for b in buffers.iter_mut() {
            if !b.release(gpu.pci) {
                all_released=false;
                info!("TU102-C: buffer release failed at {:#x}",b.dev);
            }
        }
        if all_released { info!("TU102-C: DMA cleanup complete"); }
    }
    if gpu.pci.config_read_u16(4)!=command {
        gpu.pci.cfg_write_u16(4,command);
        info!("TU102-C: PCI command restored={:#x} expected={command:#x}",gpu.pci.config_read_u16(4));
    }
}
