//! 成功 C 首轮路径的纯逻辑及 MMIO 时序；由宿主测试直接引用。
//! 参考：chenxluo/tu102-22g-uefi-unlock（MIT）及本机2026-10-29成功记录。
#![allow(dead_code)]
pub const IMAGE_SIZE: usize = 0x1bfadc8;
pub const FB: u64 = 0x580000000;
pub const FRTS: u64 = 0x57fe00000;
pub const PAYLOAD_SHA: &str = "de7288bcfb80c2d4b259a59e823c926ba0a9adcab594219cc6802c94257ce271";
pub const FWSEC_SHA: &str = "584b5fe0826c40aaa8731d70f4fe5fb505380075dca77f7a94d31cfa09d20f6e";
pub const FIRST_SLOT: u64 = 0x110000000;
pub const PLM: u32 = 0x409650;
pub const SS0: u32 = 0x409664;
pub const SS1: u32 = 0x40966c;
pub const SEC2: u32 = 0x840000;

pub fn identity(device: u16, chip: u32, crc: u32, capacity: u32) -> bool {
    device == 0x1e07 && chip == 0x162000a1 && crc == 0x2fe1e7b2 && capacity == 0x2c9
}
pub fn unlocked(a: u32, b: u32) -> bool { a == 0x88888888 && b == 8 }
pub fn heap_size(fb: u64) -> u64 {
    match fb {
        0x280000000 => 0x7f00000,
        0x500000000 => 0xfe00000,
        0x580000000 => 0x6b00000,
        _ => 0x6900000,
    }
}

pub fn slot(cursor: u64, attempt: u32, bytes: usize) -> Option<(u64, u64, usize)> {
    if cursor < FIRST_SLOT || cursor & 4095 != 0 || bytes == 0 || attempt >= 12 { return None }
    let size = (bytes as u64).checked_add(4095)? & !4095;
    let addr = cursor.checked_add(u64::from(attempt) * 0x1000000)?;
    Some((addr, addr.checked_add(size)?.checked_add(0x1000000)?, (size / 4096) as usize))
}

/// 为复现参考行为，保持其“月首+时分秒”的计时器种子；不是精确UTC转换。
/// 原C遗漏日字段，必须明确记录，不能把这个兼容步骤误称为精确实时时钟。
pub fn reference_timer_ns(year: u16, month: u8, hour: u8, minute: u8, second: u8) -> Option<u64> {
    if !(2015..2100).contains(&year) || !(1..=12).contains(&month)
        || hour > 23 || minute > 59 || second > 59 { return None }
    let leap = |y: u16| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mut days = 0u64;
    for y in 1970..year { days += 365 + u64::from(leap(y)); }
    for m in 1..month {
        days += [31,28,31,30,31,30,31,31,30,31,30,31][m as usize - 1];
        if m == 2 && leap(year) { days += 1; }
    }
    Some((days * 86400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64) * 1_000_000_000)
}

pub fn meta(radix: u64, signature: u64, boot: u64) -> Option<[u8; 256]> {
    if [radix, signature, boot].iter().any(|v| *v < 0x100000000 || v & 4095 != 0) { return None }
    let mut m = [0u8; 256];
    let values = [0xdc3aae21371a60b3, 1, radix, IMAGE_SIZE as u64, boot, 0x6000,
        0x1800, 0x800, 0, signature, 0xfa00, 0x577400000, 0x577400000, 0x100000,
        0x577500000, 0x577600000, 0x6b00000, 0x57e1f0000, 0x57fdfa000,
        FRTS, 0x100000, 0x57ff00000, FB, 0x57ff00000, 0x100000];
    for (i, v) in values.iter().enumerate() { m[i*8..i*8+8].copy_from_slice(&v.to_le_bytes()); }
    m[241] = 1;
    Some(m)
}

pub fn bl_desc(dev: u64) -> Option<[u8; 84]> {
    if dev < 0x100000000 || dev & 4095 != 0 { return None }
    let data = dev.checked_add(0x9600)?;
    let mut d = [0u8; 84];
    for (off, val) in [(0x20,4),(0x24,dev as u32),(0x28,(dev>>32) as u32),
        (0x2c,0),(0x30,0x300),(0x34,0x300),(0x38,0x9300),(0x3c,0),
        (0x40,data as u32),(0x44,(data>>32) as u32),(0x48,0x410)] {
        d[off..off+4].copy_from_slice(&val.to_le_bytes());
    }
    Some(d)
}

pub trait Mmio {
    fn read(&mut self, reg: u32) -> u32;
    fn write(&mut self, reg: u32, value: u32);
    fn stall(&mut self, microseconds: u64);
    fn flush(&mut self);
}
#[derive(Debug, PartialEq)]
pub enum Error { Entry, Load, Timeout, Selectors }
fn accessible(v: u32) -> bool { v != u32::MAX && v & 0xffff0000 != 0xbadf0000 }
pub fn pulse(io: &mut impl Mmio, base: u32) {
    for value in [1, 0] {
        io.write(base + 0x3c0, value);
        for _ in 0..16 { io.read(base + 0x3c0); }
    }
}
pub fn wait_halt(io: &mut impl Mmio, base: u32, limit: u32) -> Result<(), Error> {
    for _ in 0..limit {
        let v = io.read(base + 0x100);
        if !accessible(v) { return Err(Error::Entry) }
        if v & 0x10 != 0 { return Ok(()) }
        io.stall(100);
    }
    Err(Error::Timeout)
}
pub fn wait_scrub(io: &mut impl Mmio, base: u32, limit: u32) -> Result<(), Error> {
    for _ in 0..limit {
        let dma = io.read(base+0x10c);
        let cfg = io.read(base+0xf4);
        if !accessible(dma) || !accessible(cfg) { return Err(Error::Entry) }
        if dma&6==0 && cfg&0x1000==0 { return Ok(()) }
        io.stall(100);
    }
    Err(Error::Timeout)
}
pub fn load_imem(io: &mut impl Mmio, base: u32, dest: u32, va: u32, data: &[u8], secure: bool) {
    io.write(base+0x180, dest | 1<<24 | if secure { 1<<28 } else { 0 });
    for (i, word) in data.chunks_exact(4).enumerate() {
        if i % 64 == 0 { io.write(base+0x188, (va>>8) + (i/64) as u32); }
        io.write(base+0x184, u32::from_le_bytes(word.try_into().unwrap()));
    }
}
pub fn load_dmem(io: &mut impl Mmio, base: u32, data: &[u8]) {
    io.write(base+0x1c0, 1<<24);
    for word in data.chunks_exact(4) { io.write(base+0x1c4, u32::from_le_bytes(word.try_into().unwrap())); }
}
pub fn host_probe(io: &mut impl Mmio) -> bool {
    // 原C探测会改变可写位，不能标记为“只读”；独立身份门控通过后才调用。
    for (reg, value) in [(PLM,u32::MAX),(SS0,0x88888888),(SS1,8)] {
        io.write(reg, value); io.read(reg);
    }
    if unlocked(io.read(SS0), io.read(SS1)) { return true }
    for (reg, value) in [(PLM,0),(PLM,0xfffffffe),(SS0,0x11111111),(SS1,0)] {
        io.write(reg, value); io.read(reg);
    }
    false
}
pub fn fire_direct(io: &mut impl Mmio, image: &[u8], address: u64) -> Result<(), Error> {
    if image.len() != 0xe700 || address < 0x100000000 || address & 4095 != 0 { return Err(Error::Load) }
    let cpu = io.read(SEC2+0x100);
    if !accessible(cpu) || cpu & 0x10 == 0 { return Err(Error::Entry) }
    let ctl = io.read(SEC2+0x624);
    if !accessible(ctl) { return Err(Error::Entry) }
    io.write(SEC2+0x624, ctl | 1<<7);
    io.write(SEC2+0x10c, 0);
    // 与成功首轮一致：不复位SEC2，也不重写其TRANSCFG。
    load_imem(io, SEC2, 0, 0, &image[..0x100], false);
    load_imem(io, SEC2, 0x100, 0x100, &image[0x100..0x8500], true);
    load_dmem(io, SEC2, &image[0x8500..]);
    io.flush();
    io.write(SEC2+0x180, 0);
    if io.read(SEC2+0x184) != u32::from_le_bytes(image[..4].try_into().unwrap()) { return Err(Error::Load) }
    io.write(SEC2+0x1c0, 0x200);
    if io.read(SEC2+0x1c4) != u32::from_le_bytes(image[0x8700..0x8704].try_into().unwrap()) { return Err(Error::Load) }
    io.write(SEC2+0x104, 0);
    io.write(SEC2+0x40, address as u32);
    io.write(SEC2+0x44, (address>>32) as u32);
    io.flush();
    io.write(SEC2+0x100, 2);
    io.stall(3_000_000); // 静默窗口内无读写采样
    wait_halt(io, SEC2, 20_000)?;
    io.stall(1_000_000);
    if !unlocked(io.read(SS0), io.read(SS1)) { return Err(Error::Selectors) }
    Ok(())
}
pub fn sanitize(io: &mut impl Mmio) -> Result<(), Error> {
    wait_halt(io, SEC2, 20_000)?;
    pulse(io, SEC2);
    io.stall(100_000);
    io.write(SEC2+0x40, 0);
    io.write(SEC2+0x44, 0);
    pulse(io, SEC2);
    if !unlocked(io.read(SS0), io.read(SS1)) { return Err(Error::Selectors) }
    Ok(())
}
