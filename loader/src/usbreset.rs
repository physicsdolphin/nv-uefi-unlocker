//! CONFIRM 之前可选的"按设备"USB 端口复位。
//!
//! 启动卷上的 `\input-reset.txt` 每行列出一个设备:
//!     usb 0C45:7E0C        (十六进制 VID:PID,`#` 起注释)
//! 没有该文件时什么都不做。列出的设备存在时调用一次
//! `EFI_USB_IO_PROTOCOL.UsbPortReset()`,由 USB 总线驱动重新枚举。
//! 不复位主控制器,不碰其他设备。只在中止倒计时打开之前运行一次。
//! 每个 USB 接口会记录一行 `USB-INV`,用于确定是哪个设备在上报按键。
//! 注意:对带键盘接口的鼠标,端口复位不一定能消除被持续上报的键
//! (已在一台设备上观察到);此时由 settle_input_once 的"幽灵键抑制"兜底。

use alloc::vec::Vec;
use core::time::Duration;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::proto::usb::io::UsbIo;
use uefi::Identify;

/// 解析 `usb VVVV:PPPP` 行,坏行跳过。
pub fn parse(content: &[u8]) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    for raw in content.split(|&b| b == b'\n') {
        let line = match raw.iter().position(|&b| b == b'#') {
            Some(i) => &raw[..i],
            None => raw,
        };
        let Ok(text) = core::str::from_utf8(line) else { continue };
        let mut it = text.split_whitespace();
        if it.next().map(|w| w.eq_ignore_ascii_case("usb")) != Some(true) {
            continue;
        }
        let Some(ids) = it.next() else { continue };
        let Some((v, p)) = ids.split_once(':') else { continue };
        if let (Ok(v), Ok(p)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) {
            if !out.contains(&(v, p)) {
                out.push((v, p));
            }
        }
    }
    out
}

/// 对每个列出且存在的设备复位一次(复合设备每个接口一个 UsbIo 句柄,
/// 第一个匹配的句柄复位整个设备)。
pub fn reset_listed(list: &[(u16, u16)]) {
    // 总线可能仍在枚举:最多重试 3 秒。
    let mut found = None;
    for attempt in 0..15 {
        if let Ok(h) = boot::locate_handle_buffer(SearchType::ByProtocol(&UsbIo::GUID)) {
            found = Some(h);
            break;
        }
        if attempt < 14 {
            boot::stall(Duration::from_millis(200));
        }
    }
    let Some(handles) = found else {
        info!("USB-RESET: no UsbIo handles after 3 s");
        return;
    };
    info!("USB-RESET: {} UsbIo handle(s), {} listed device(s)", handles.len(), list.len());
    let mut done: Vec<(u16, u16)> = Vec::new();
    let mut any = false;
    for &h in handles.iter() {
        // SAFETY: GetProtocol 不改变 USB 总线驱动对设备的所有权。
        let Ok(mut usb) = (unsafe {
            boot::open_protocol::<UsbIo>(
                OpenProtocolParams { handle: h, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        let Ok(d) = usb.device_descriptor() else { continue };
        let id = (d.id_vendor, d.id_product);
        if let Ok(i) = usb.interface_descriptor() {
            info!(
                "USB-INV: {:04X}:{:04X} iface#{} class={:02X} sub={:02X} proto={:02X}{}",
                id.0, id.1, i.interface_number, i.interface_class, i.interface_subclass, i.interface_protocol,
                if i.interface_class == 3 && i.interface_protocol == 1 { " (HID boot KEYBOARD)" }
                else if i.interface_class == 3 && i.interface_protocol == 2 { " (HID boot MOUSE)" } else { "" }
            );
        }
        if !list.contains(&id) || done.contains(&id) {
            continue;
        }
        done.push(id);
        match usb.port_reset() {
            Ok(()) => info!("USB-RESET: {:04X}:{:04X} port reset ok", id.0, id.1),
            Err(e) => info!("USB-RESET: {:04X}:{:04X} port reset failed: {:?}", id.0, id.1, e.status()),
        }
        any = true;
    }
    if !any {
        info!("USB-RESET: none of the listed devices is present");
        return;
    }
    // 在安静窗口之前等待总线驱动重新枚举。
    boot::stall(Duration::from_millis(1500));
    info!("USB-RESET: re-enumeration wait done");
}
