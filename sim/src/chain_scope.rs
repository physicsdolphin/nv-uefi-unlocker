//! Boot-option device-scope parsing + matching: reference implementation
//! and shape tests (issue #128).
//!
//! `loader/src/chainload.rs`'s `parse_option_device_scope`, `OptionScope`,
//! `hd_identity_match_in`, `node_at`, `first_divergent_node`,
//! `strip_end_node` and `sfs_handle_matches_prefix` must stay textually
//! identical to the copies here (loader is UEFI-target-only and cannot run
//! host tests; sim is the test bench).
//!
//! The shapes are the real ones from the report: run 17 (a short-form HD
//! option - one 42-byte HD node - against full PciRoot/.../HD volume
//! paths), run 16 (full-path options whose stored prefix matched no live
//! volume), run 14/15 (bulk handling behind the "no resolvable firmware
//! option" line), and the QEMU control's bulk entry, which the empty-path
//! branch used to feed to `Dir::open("")` as a file name.

const NODE_TYPE_MEDIA: u8 = 0x04;
const NODE_SUBTYPE_HD: u8 = 0x01;
const NODE_SUBTYPE_FILE: u8 = 0x04;
const NODE_TYPE_END: u8 = 0x7f;

/// Decode UCS-2LE code units until NUL (or cap) into a lossy String.
fn ucs2_lossy_prefix(bytes: &[u8], max_chars: usize) -> String {
    let mut s = String::new();
    let mut i = 0usize;
    while i + 1 < bytes.len() && s.chars().count() < max_chars {
        let c = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if c == 0 {
            break;
        }
        s.push(char::from_u32(c as u32).unwrap_or('?'));
        i += 2;
    }
    s
}

/// The device scope of a boot option's FilePathList (issue #128). Three
/// shapes reach real firmware; two of them are "short forms" (UEFI 2.10
/// §3.3) that never align with a live volume path node 0 and therefore
/// cannot be resolved by prefix comparison:
///
/// - `Volume`: full device path prefix (head-aligned). An empty `path` is
///   the bulk form - firmware-created volume entries carrying no FILE node
///   (the QEMU/OVMF control showed the shape; bcdedit's "path" column is
///   not part of the NVRAM option): the firmware chose the VOLUME, the
///   loader finds the loader file among its candidates there;
/// - `ShortFormHd`: the list starts at an HD node - exactly what
///   `bcfg boot addp` writes ("Only the portion of the device path
///   starting with the hard drive partition is placed in the option").
///   Firmware expands it in `BmExpandPartitionDevicePath` by partition
///   identity, never by alignment. run17 (#128) is this shape: a 42-byte
///   single HD node against 66-82-byte volume paths;
/// - `ShortFormFile`: a bare FilePath node - firmware searches every
///   volume for it (`BmExpandFileDevicePath`).
///
/// None only when the list is malformed or empty.
pub enum OptionScope {
    Volume { prefix: Vec<u8>, path: Vec<u8> },
    ShortFormHd { hd: Vec<u8>, tail: Vec<u8>, path: Vec<u8> },
    ShortFormFile { path: Vec<u8> },
}

pub fn parse_option_device_scope(data: &[u8]) -> Option<OptionScope> {
    let mut i = 4usize; // attributes
    while i + 1 < data.len() {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        if c == 0 {
            break;
        }
    }
    let list = i;
    // Node boundaries up to END so the shape can be classified before the
    // FILE split: a leading FILE node means a bare short form, not an
    // empty device prefix.
    let mut nodes: Vec<(usize, usize)> = Vec::new();
    let mut off = i;
    while off + 4 <= data.len() {
        let t = data[off];
        let len = u16::from_le_bytes([data[off + 2], data[off + 3]]) as usize;
        if len < 4 || off + len > data.len() {
            return None;
        }
        if t == NODE_TYPE_END {
            break;
        }
        nodes.push((off, off + len));
        off += len;
    }
    if nodes.is_empty() {
        return None;
    }
    let file_node = nodes
        .iter()
        .position(|(s, _)| data[*s] == NODE_TYPE_MEDIA && data[*s + 1] == NODE_SUBTYPE_FILE);
    if file_node == Some(0) {
        let (s, e) = nodes[0];
        let path = ucs2_lossy_prefix(&data[s + 4..e], 240);
        if path.is_empty() || !path.starts_with('\\') {
            return None;
        }
        return Some(OptionScope::ShortFormFile { path: path.into_bytes() });
    }
    let (device_end, path) = match file_node {
        Some(k) => {
            let (s, e) = nodes[k];
            let path = ucs2_lossy_prefix(&data[s + 4..e], 240);
            if path.is_empty() || !path.starts_with('\\') {
                return None;
            }
            (s, path.into_bytes())
        }
        // No FILE node: the whole node list is the device scope (bulk).
        None => (off, Vec::new()),
    };
    if device_end <= list {
        return None;
    }
    if data[list] == NODE_TYPE_MEDIA && data[list + 1] == NODE_SUBTYPE_HD {
        let hd_end = nodes[0].1;
        return Some(OptionScope::ShortFormHd {
            hd: data[list..hd_end].to_vec(),
            tail: data[hd_end..device_end].to_vec(),
            path,
        });
    }
    Some(OptionScope::Volume { prefix: data[list..device_end].to_vec(), path })
}

/// HD node identity match, EDK2 `BmMatchPartitionDevicePathNode`
/// semantics (MdeModulePkg/Library/UefiBootManagerLib/BmBoot.c): walk the
/// WHOLE volume path for an HD node and compare the four identity fields
/// only - partition number, MBR type, signature type, 16-byte signature.
/// Start/size are not identity (they move when a partition is resized or
/// re-created; firmware never compares them). GPT signatures are the
/// partition GUID, not the disk GUID.
pub fn hd_identity_match_in(bytes: &[u8], option_hd: &[u8], option_tail: &[u8]) -> bool {
    // 4 header + 4 part + 8 start + 8 size + 16 sig + 2 type bytes.
    const HD_LEN: usize = 42;
    if option_hd.len() < HD_LEN {
        return false;
    }
    let vol = strip_end_node(bytes);
    let mut off = 0usize;
    while off + 4 <= vol.len() {
        let len = u16::from_le_bytes([vol[off + 2], vol[off + 3]]) as usize;
        if len < 4 || off + len > vol.len() {
            return false;
        }
        if vol[off] == NODE_TYPE_MEDIA && vol[off + 1] == NODE_SUBTYPE_HD && len >= HD_LEN {
            let node = &vol[off..off + len];
            // Nodes the option carries AFTER the HD node (nested
            // partitions) must line up with the volume's remainder; the
            // empty tail is the normal case and trivially true.
            if node[4..8] == option_hd[4..8]
                && node[24..40] == option_hd[24..40]
                && node[40] == option_hd[40]
                && node[41] == option_hd[41]
                && vol[off + len..].starts_with(option_tail)
            {
                return true;
            }
        }
        off += len;
    }
    false
}

/// Drop a trailing END-of-node sequence (any 0x7F-type node at the tail).
pub fn strip_end_node(bytes: &[u8]) -> &[u8] {
    let mut off = 0usize;
    while off + 4 <= bytes.len() {
        let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
        if len < 4 || off + len > bytes.len() {
            return &bytes[..off];
        }
        if bytes[off] == NODE_TYPE_END {
            return &bytes[..off];
        }
        off += len;
    }
    &bytes[..off]
}

/// `sfs_handle_matches_prefix` minus the handle: the volume path (END
/// stripped) must be the node-aligned prefix of the option's prefix.
pub fn volume_matches_prefix(volume_path_with_end: &[u8], prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return false;
    }
    let vol = strip_end_node(volume_path_with_end);
    !vol.is_empty() && vol.len() <= prefix.len() && vol == &prefix[..vol.len()]
}

/// One device-path node at `off`: (type, subtype, payload).
pub fn node_at(bytes: &[u8], off: usize) -> Option<(u8, u8, &[u8])> {
    if off + 4 > bytes.len() {
        return None;
    }
    let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
    if len < 4 || off + len > bytes.len() {
        return None;
    }
    Some((bytes[off], bytes[off + 1], &bytes[off + 4..off + len]))
}

/// Ordinal of the first node where two node sequences differ; equal
/// sequences diverge at their shared node count.
pub fn first_divergent_node(a: &[u8], b: &[u8]) -> usize {
    let (mut i, mut j, mut n) = (0usize, 0usize, 0usize);
    loop {
        match (node_at(a, i), node_at(b, j)) {
            (Some((ta, sa, pa)), Some((tb, sb, pb))) => {
                if ta != tb || sa != sb || pa != pb {
                    return n;
                }
                i += 4 + pa.len();
                j += 4 + pb.len();
                n += 1;
            }
            _ => return n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end() -> Vec<u8> {
        vec![0x7f, 0xff, 0x04, 0x00]
    }

    fn pci(dev: u8, func: u8) -> Vec<u8> {
        vec![0x01, 0x01, 0x06, 0x00, dev, func]
    }

    /// HD node: part number + start/size + 16-byte signature + type bytes.
    fn hd(part: u32, sig: [u8; 16], sig_type: u8) -> Vec<u8> {
        let mut n = vec![0x04, 0x01, 0x2a, 0x00];
        n.extend_from_slice(&part.to_le_bytes());
        n.extend_from_slice(&[0x11; 8]); // start
        n.extend_from_slice(&[0x22; 8]); // size
        n.extend_from_slice(&sig);
        n.push(0x02); // MBRType: GPT
        n.push(sig_type);
        n
    }

    /// HD node with distinct start/size - same identity, other geometry.
    fn hd_geometry(part: u32, sig: [u8; 16], sig_type: u8) -> Vec<u8> {
        let mut n = hd(part, sig, sig_type);
        n[8..24].fill(0x33);
        n
    }

    fn file(path: &str) -> Vec<u8> {
        let units: Vec<u16> = path.encode_utf16().chain(core::iter::once(0)).collect();
        let len = (4 + 2 * units.len()) as u16;
        let mut n = vec![0x04, 0x04, len as u8, (len >> 8) as u8];
        for u in units {
            n.extend_from_slice(&u.to_le_bytes());
        }
        n
    }

    fn option(nodes: &[&[u8]]) -> Vec<u8> {
        let mut d = vec![0u8; 4]; // attributes
        d.extend_from_slice("Windows Boot Manager".encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>().as_slice());
        d.extend_from_slice(&[0, 0]);
        for n in nodes {
            d.extend_from_slice(n);
        }
        d.extend_from_slice(&end());
        d
    }

    const SIG: [u8; 16] = [0x11; 16];

    fn device() -> Vec<u8> {
        [pci(0x01, 0x00), hd(2, SIG, 0x02)].concat()
    }

    /// A full volume path: controller chain, HD last, END appended.
    fn volume(part: u32, sig: [u8; 16]) -> Vec<u8> {
        [pci(0x01, 0x00), pci(0x00, 0x00), hd(part, sig, 0x02), end()].concat()
    }

    #[test]
    fn file_option_splits_prefix_and_path() {
        let data = option(&[&device(), &file(r"\EFI\Microsoft\Boot\bootmgfw.efi")]);
        let OptionScope::Volume { prefix, path } = parse_option_device_scope(&data).unwrap() else {
            panic!("full path must parse as Volume")
        };
        assert_eq!(prefix, device());
        assert_eq!(path, br"\EFI\Microsoft\Boot\bootmgfw.efi".to_vec());
    }

    #[test]
    fn bulk_option_returns_empty_path() {
        // run 14/15/16 shape: the firmware option names the volume only.
        let data = option(&[&device()]);
        let OptionScope::Volume { prefix, path } = parse_option_device_scope(&data).unwrap() else {
            panic!("full bulk path must parse as Volume")
        };
        assert_eq!(prefix, device());
        assert!(path.is_empty(), "bulk entries must NOT fake a file path");
    }

    #[test]
    fn empty_path_must_not_reach_dir_open() {
        let data = option(&[&device()]);
        let OptionScope::Volume { path, .. } = parse_option_device_scope(&data).unwrap() else {
            panic!("must parse")
        };
        assert_eq!(path.len(), 0, "bulk entry must not carry a file path");
    }

    #[test]
    fn short_form_hd_option_parses_hd_and_path() {
        // run 17 (#128) shape: node 0 IS the HD node, no controller chain.
        let node = hd(1, SIG, 0x02);
        let data = option(&[&node, &file(r"\EFI\Microsoft\Boot\bootmgfw.efi")]);
        let OptionScope::ShortFormHd { hd, tail, path } = parse_option_device_scope(&data).unwrap() else {
            panic!("HD-first option must parse as ShortFormHd")
        };
        assert_eq!(hd, node);
        assert_eq!(hd.len(), 42);
        assert!(tail.is_empty());
        assert_eq!(path, br"\EFI\Microsoft\Boot\bootmgfw.efi".to_vec());
    }

    #[test]
    fn short_form_hd_bulk_option_has_empty_path() {
        let node = hd(1, SIG, 0x02);
        let data = option(&[&node]);
        let OptionScope::ShortFormHd { path, .. } = parse_option_device_scope(&data).unwrap() else {
            panic!("HD-only option must parse as ShortFormHd")
        };
        assert!(path.is_empty(), "HD-only is the bulk short form");
    }

    #[test]
    fn short_form_file_option_parses() {
        // Firmware's file-path-only short form (bcfg addp on a pathless
        // device / network-less entries): no device at all.
        let data = option(&[&file(r"\EFI\BOOT\BOOTX64.EFI")]);
        let OptionScope::ShortFormFile { path } = parse_option_device_scope(&data).unwrap() else {
            panic!("FILE-first option must parse as ShortFormFile")
        };
        assert_eq!(path, br"\EFI\BOOT\BOOTX64.EFI".to_vec());
    }

    #[test]
    fn hd_identity_ignores_start_and_size() {
        // Firmware compares four identity fields only; a resized or
        // re-created partition keeps its GUID and must still match.
        let opt = hd(1, SIG, 0x02);
        let vol = [pci(0x01, 0x00), hd_geometry(1, SIG, 0x02)].concat();
        assert!(hd_identity_match_in(&vol, &opt, &[]));
    }

    #[test]
    fn hd_identity_partition_number_differs() {
        let opt = hd(1, SIG, 0x02);
        assert!(!hd_identity_match_in(&[pci(0x01, 0x00), hd(2, SIG, 0x02)].concat(), &opt, &[]));
    }

    #[test]
    fn hd_identity_signature_differs() {
        let opt = hd(1, SIG, 0x02);
        let mut other = SIG;
        other[3] ^= 0x40;
        assert!(!hd_identity_match_in(&[pci(0x01, 0x00), hd(1, other, 0x02)].concat(), &opt, &[]));
    }

    #[test]
    fn hd_identity_signature_type_differs() {
        // GPT vs MBR signature basis is identity, not decoration.
        let opt = hd(1, SIG, 0x02);
        assert!(!hd_identity_match_in(&[pci(0x01, 0x00), hd(1, SIG, 0x01)].concat(), &opt, &[]));
    }

    #[test]
    fn hd_identity_mbr_type_differs() {
        let opt = hd(1, SIG, 0x02);
        let mut vol = [pci(0x01, 0x00), hd(1, SIG, 0x02)].concat();
        let hd_off = 6;
        vol[hd_off + 40] = 0x01; // MBRType
        assert!(!hd_identity_match_in(&vol, &opt, &[]));
    }

    #[test]
    fn hd_identity_volume_without_hd_node() {
        // OVMF-style volume path: PciRoot/Pci/NVMe, no HD node.
        let vol = [pci(0x01, 0x00), pci(0x00, 0x00)].concat();
        assert!(!hd_identity_match_in(&vol, &hd(1, SIG, 0x02), &[]));
    }

    #[test]
    fn hd_identity_walks_whole_volume_path() {
        // Nested partition: the matching HD node is not the last node.
        let vol = [pci(0x01, 0x00), hd(1, SIG, 0x02), hd(4, SIG, 0x02)].concat();
        assert!(hd_identity_match_in(&vol, &hd(4, SIG, 0x02), &[]));
        assert!(hd_identity_match_in(&vol, &hd(1, SIG, 0x02), &[]));
        assert!(!hd_identity_match_in(&vol, &hd(9, SIG, 0x02), &[]));
    }

    #[test]
    fn hd_identity_tail_must_line_up() {
        // Nested partition encoded in the option: the second HD node has to
        // line up with the volume's node after the first match.
        let inner = hd(4, SIG, 0x02);
        let outer = hd(1, SIG, 0x02);
        let vol = [pci(0x01, 0x00), outer.clone(), inner.clone()].concat();
        assert!(hd_identity_match_in(&vol, &outer, &inner));
        assert!(!hd_identity_match_in(&vol, &outer, &hd(9, SIG, 0x02)));
    }

    #[test]
    fn hd_identity_rejects_truncated_option_node() {
        let opt = hd(1, SIG, 0x02)[..41].to_vec();
        assert!(!hd_identity_match_in(&volume(1, SIG), &opt, &[]));
    }

    #[test]
    fn fv_only_entry_parses_to_volume_scope() {
        // UiApp-style entry: one FvFile node, no media volume.
        let mut fv = vec![0x04, 0x06, 0x12, 0x00];
        fv.extend_from_slice(&[0x33; 14]); // GUID + type
        let data = option(&[&fv]);
        let OptionScope::Volume { prefix, path } = parse_option_device_scope(&data).unwrap() else {
            panic!("FV entry must stay a Volume scope")
        };
        assert_eq!(prefix, fv);
        assert!(path.is_empty());
    }

    #[test]
    fn empty_path_list_is_none() {
        assert!(parse_option_device_scope(&option(&[])).is_none());
    }

    #[test]
    fn malformed_or_pathless_file_is_none() {
        let mut broken = device();
        broken.extend_from_slice(&[0x04, 0x04, 0x02, 0x00]); // len 2
        assert!(parse_option_device_scope(&option(&[&broken])).is_none());

        // Relative path.
        let data = option(&[&device(), &file(r"EFI\BOOT\BOOTX64.EFI")]);
        assert!(parse_option_device_scope(&data).is_none());
    }

    #[test]
    fn prefix_match_ignores_trailing_end_node() {
        let vol = [device(), end()].concat();
        let prefix = device();
        assert!(volume_matches_prefix(&vol, &prefix));
    }

    #[test]
    fn short_form_hd_never_matches_by_prefix() {
        // The run17 fact: 42 B HD-first option vs 66-82 B volume paths.
        // Head alignment must fail for every volume - that is why the
        // suffix/identity branch exists.
        let OptionScope::ShortFormHd { hd, .. } = parse_option_device_scope(&option(&[&hd(1, SIG, 0x02), &file(r"\x")])).unwrap() else {
            panic!("must be ShortFormHd")
        };
        assert!(!volume_matches_prefix(&volume(1, SIG), &hd));
        assert!(hd_identity_match_in(&volume(1, SIG), &hd, &[]));
        // And the full-path function still matches full paths.
        assert!(volume_matches_prefix(&[device(), end()].concat(), &device()));
    }

    #[test]
    fn different_partition_signature_does_not_match() {
        let mut other_sig = SIG;
        other_sig[0] ^= 0x01;
        let vol = [[pci(0x01, 0x00), hd(2, other_sig, 0x02)].concat(), end()].concat();
        assert!(!volume_matches_prefix(&vol, &device()));
    }

    #[test]
    fn volume_longer_than_prefix_does_not_match() {
        // A volume carrying an extra node the stored option lacks.
        let vol = [[pci(0x01, 0x00), pci(0x00, 0x00), hd(2, SIG, 0x02)].concat(), end()].concat();
        assert!(!volume_matches_prefix(&vol, &device()));
    }

    #[test]
    fn divergence_names_the_first_different_node() {
        assert_eq!(first_divergent_node(&device(), &device()), 2);
        let mut other = device();
        other[4] ^= 0x01; // inside the PCI node payload (node 0)
        assert_eq!(first_divergent_node(&device(), &other), 0);
        // Shorter sequence diverges at its own length.
        assert_eq!(first_divergent_node(&device(), &pci(0x01, 0x00)), 1);
    }
}
