//! 对生产MMIO时序做负控；不模拟NVIDIA固件内部执行或声称真机兼容。
use std::collections::BTreeMap;
use crate::tu102_profile::{self as p, Mmio, Error, SEC2, SS0, SS1, PLM};

#[derive(Default)]
struct Io {
    regs: BTreeMap<u32,u32>, imem: BTreeMap<u32,u32>, dmem: BTreeMap<u32,u32>,
    ip: u32, dp: u32, events: Vec<(u8,u32,u64)>, effect: u8, running: bool,
    status: Option<u32>,
}
impl Io {
    fn locked() -> Self {
        let mut s=Self::default();
        for (r,v) in [(SEC2+0x100,0x10),(SEC2+0x624,0x110),(SS0,0x410000),(SS1,3),(PLM,0xffffff8f)] { s.regs.insert(r,v); }
        s
    }
}
impl Mmio for Io {
    fn read(&mut self,r:u32)->u32 {
        self.events.push((0,r,0));
        if r==SEC2+0x184 { return *self.imem.get(&self.ip).unwrap_or(&0) }
        if r==SEC2+0x1c4 { return *self.dmem.get(&self.dp).unwrap_or(&0) }
        *self.regs.get(&r).unwrap_or(&0)
    }
    fn write(&mut self,r:u32,v:u32) {
        self.events.push((1,r,v as u64));
        match r {
            x if x==SEC2+0x180 => self.ip=v&0xfffffc,
            x if x==SEC2+0x184 => { self.imem.insert(self.ip,v); self.ip+=4; },
            x if x==SEC2+0x1c0 => self.dp=v&0xfffffc,
            x if x==SEC2+0x1c4 => { self.dmem.insert(self.dp,v); self.dp+=4; },
            x if x==SEC2+0x100 && v==2 => {
                self.regs.insert(r,if self.running {0}else{0x10});
                if self.effect>=1 { self.regs.insert(SS0,0x88888888); }
                if self.effect==2 { self.regs.insert(SS1,8); }
                self.regs.insert(SEC2+0x40,self.status.unwrap_or(if self.effect==2 {0}else{0x91}));
            },
            PLM => { if v!=u32::MAX { self.regs.insert(PLM,0xfffffe8e); } },
            SS0 => { if v==0x11111111 { self.regs.insert(SS0,0); } },
            SS1 => { if v==0 { self.regs.insert(SS1,0); } },
            _ => { self.regs.insert(r,v); },
        }
    }
    fn stall(&mut self,us:u64) { self.events.push((2,0,us)); }
    fn flush(&mut self) { self.events.push((3,0,0)); }
}
fn image()->Vec<u8> {
    let mut image=vec![0u8;0xe700];
    image[..4].copy_from_slice(&0xce100049u32.to_le_bytes());
    image[0x8700..0x8704].copy_from_slice(&0x67436032u32.to_le_bytes());
    image
}

#[test]
fn exact_c_meta_matches_independently_compiled_c_fixture() {
    let text=include_str!("../tests/fixtures/tu102-c-meta.hex").trim();
    let expected:Vec<u8>=(0..text.len()).step_by(2).map(|i|u8::from_str_radix(&text[i..i+2],16).unwrap()).collect();
    let meta=p::meta(0x115c20000,0x110000000,0x11201f000).unwrap();
    assert_eq!(meta.as_slice(),expected.as_slice());
    assert_eq!(meta[241],1);
}
#[test]
fn selectors_survive_relocked_plm_and_start_has_quiet_window() {
    let mut io=Io::locked(); io.effect=2;
    assert_eq!(p::fire_direct(&mut io,&image(),0x11882b000),Ok(()));
    assert_eq!(io.regs[&PLM],0xffffff8f);
    let starts:Vec<_>=io.events.iter().enumerate().filter(|(_,e)|**e==(1,SEC2+0x100,2)).collect();
    assert_eq!(starts.len(),1);
    assert_eq!(io.events[starts[0].0+1],(2,0,3_000_000));
    assert!(!io.events.iter().any(|e|e.0==1 && [SEC2+0x3c0,SEC2+0x600].contains(&e.1)));
}
#[test]
fn clean_halt_and_mailbox_zero_without_ss_are_failures() {
    let mut io=Io::locked();
    io.status=Some(0);
    assert_eq!(p::fire_direct(&mut io,&image(),0x11882b000),Err(Error::Selectors));
    assert_eq!(io.regs[&(SEC2+0x40)],0);
    assert_eq!(io.events.iter().filter(|e|**e==(1,SEC2+0x100,2)).count(),1);
}
#[test]
fn partial_ss_write_is_not_success() {
    let mut io=Io::locked(); io.effect=1;
    assert_eq!(p::fire_direct(&mut io,&image(),0x11882b000),Err(Error::Selectors));
}
#[test]
fn nonhalting_or_inaccessible_engine_never_becomes_success() {
    let mut io=Io::locked();io.effect=2;io.running=true;
    assert_eq!(p::fire_direct(&mut io,&image(),0x11882b000),Err(Error::Timeout));
    let mut bad=Io::locked();bad.regs.insert(SEC2+0x100,0xbadf5040);
    assert_eq!(p::fire_direct(&mut bad,&image(),0x11882b000),Err(Error::Entry));
    assert!(!bad.events.iter().any(|e|e.0==1));
}
#[test]
fn bad_dma_or_image_rejected_before_any_write() {
    for (n,addr) in [(0xe6ff,0x11882b000),(0xe700,0x80000000),(0xe700,0x11882b001)] {
        let mut io=Io::locked();
        assert_eq!(p::fire_direct(&mut io,&vec![0;n],addr),Err(Error::Load));
        assert!(io.events.is_empty());
    }
}
#[test]
fn gsp_completion_and_scrub_waits_are_bounded_and_fail_closed() {
    let mut io=Io::locked();
    assert_eq!(p::wait_halt(&mut io,0x110000,3),Err(Error::Timeout));
    assert_eq!(io.events.iter().filter(|e|e.0==2).count(),3);
    io.regs.insert(0x110100,0x10);
    assert_eq!(p::wait_halt(&mut io,0x110000,3),Ok(()));
    io.regs.insert(0x11010c,2);
    assert_eq!(p::wait_scrub(&mut io,0x110000,3),Err(Error::Timeout));
    io.regs.insert(0x11010c,0);
    assert_eq!(p::wait_scrub(&mut io,0x110000,3),Ok(()));
}
#[test]
fn cleanup_does_not_overwrite_selectors_or_open_plm() {
    let mut io=Io::locked();io.regs.insert(SS0,0x88888888);io.regs.insert(SS1,8);
    assert_eq!(p::sanitize(&mut io),Ok(()));
    assert!(!io.events.iter().any(|e|e.0==1 && [SS0,SS1,PLM].contains(&e.1)));
}
#[test]
fn identity_and_allocator_do_not_accept_other_cards_or_low_fallback() {
    for (gib,size) in [(8,0x6900000),(10,0x7f00000),(11,0x6900000),(20,0xfe00000),(22,0x6b00000)] {
        assert_eq!(p::heap_size(gib<<30),size);
    }
    assert!(p::identity(0x1e07,0x162000a1,0x2fe1e7b2,0x2c9));
    assert!(!p::identity(0x1e04,0x162000a1,0x2fe1e7b2,0x2c9));
    assert!(!p::identity(0x1e07,0x162000a1,0,0x2c9));
    assert!(!p::identity(0x1e07,0x162000a1,0x2fe1e7b2,0x100));
    let (sig,next,n)=p::slot(p::FIRST_SLOT,0,0xfa00).unwrap();
    assert_eq!((sig,next,n),(0x110000000,0x111010000,16));
    assert!(p::slot(next,12,0x1000).is_none());
    assert!(p::slot(0x100000000,0,0x1000).is_none());
    assert!(p::slot(u64::MAX&!4095,0,1).is_none());
    assert!(p::meta(0x80000000,0x110000000,0x11201f000).is_none());
}
#[test]
fn timer_seed_preserves_reference_month_start_semantics() {
    let ns=p::reference_timer_ns(2026,10,4,12,14).unwrap();
    assert_eq!(ns>>32,0x18da4c84); // 实际C成功日志中的高32位
    assert!(p::reference_timer_ns(2026,0,0,0,0).is_none());
    assert!(p::reference_timer_ns(2026,13,0,0,0).is_none());
    assert!(p::reference_timer_ns(2026,1,24,0,0).is_none());
}
#[test]
fn host_probe_side_effects_are_preserved_and_not_claimed_read_only() {
    let mut io=Io::locked();
    assert!(!p::host_probe(&mut io));
    let writes:Vec<_>=io.events.iter().filter(|e|e.0==1).map(|e|(e.1,e.2 as u32)).collect();
    assert_eq!(writes,vec![(PLM,u32::MAX),(SS0,0x88888888),(SS1,8),
        (PLM,0),(PLM,0xfffffffe),(SS0,0x11111111),(SS1,0)]);
    assert_eq!((io.regs[&SS0],io.regs[&SS1]),(0,0));
}
