//! Boot-option device-scope parsing + prefix matching: reference
//! implementation + shape tests (issue #128).
//!
//! `loader/src/chainload.rs`'s `parse_option_device_scope`, `node_at`,
//! `first_divergent_node`, `strip_end_node` and `sfs_handle_matches_prefix`
//! must stay textually identical to the copies here (loader is
//! UEFI-target-only and cannot run host tests; sim is the test bench).
//!
//! The shapes are the real ones from the report: run 14/15 (bulk handling
//! behind the "no resolvable firmware option" line), run 16 (FILE-node
//! options whose stored prefix matched no live volume), and the QEMU
//! control's bulk entry, which the empty-path branch used to feed to
//! `Dir::open("")` as a file name.

const NODE_TYPE_MEDIA: u8 = 0x04;
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

/// Split a boot option's FilePathList into (device prefix, optional FILE
/// path). Bulk entries - firmware-created volume entries carrying no FILE
/// node (the QEMU/OVMF control showed the shape; bcdedit's "path" column
/// is not part of the NVRAM option) - return the whole node list as the
/// prefix with an empty path. The FILE form keeps its exact path. None
/// only when the list is malformed or empty.
pub fn parse_option_device_scope(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut i = 4usize; // attributes
    while i + 1 < data.len() {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        if c == 0 {
            break;
        }
    }
    let mut off = i;
    while off + 4 <= data.len() {
        let (t, st) = (data[off], data[off + 1]);
        let len = u16::from_le_bytes([data[off + 2], data[off + 3]]) as usize;
        if len < 4 || off + len > data.len() {
            return None;
        }
        if t == NODE_TYPE_END {
            break;
        }
        if t == NODE_TYPE_MEDIA && st == NODE_SUBTYPE_FILE {
            let prefix = data[i..off].to_vec();
            let path = ucs2_lossy_prefix(&data[off + 4..off + len], 240);
            if prefix.is_empty() || path.is_empty() || !path.starts_with('\\') {
                return None;
            }
            return Some((prefix, path.into_bytes()));
        }
        off += len;
    }
    let prefix = data[i..off].to_vec();
    if prefix.is_empty() {
        return None;
    }
    Some((prefix, Vec::new()))
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

    /// HD node: part number + 16-byte signature + type + reserved.
    fn hd(part: u32, sig: [u8; 16], sig_type: u8) -> Vec<u8> {
        let mut n = vec![0x04, 0x01, 0x2a, 0x00];
        n.extend_from_slice(&part.to_le_bytes());
        n.extend_from_slice(&[0u8; 16]); // start/end
        n.extend_from_slice(&sig);
        n.push(sig_type);
        n.push(0);
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

    #[test]
    fn file_option_splits_prefix_and_path() {
        let data = option(&[&device(), &file(r"\EFI\Microsoft\Boot\bootmgfw.efi")]);
        let (prefix, path) = parse_option_device_scope(&data).unwrap();
        assert_eq!(prefix, device());
        assert_eq!(path, br"\EFI\Microsoft\Boot\bootmgfw.efi".to_vec());
    }

    #[test]
    fn bulk_option_returns_empty_path() {
        // run 14/15/16 shape: the firmware option names the volume only.
        let data = option(&[&device()]);
        let (prefix, path) = parse_option_device_scope(&data).unwrap();
        assert_eq!(prefix, device());
        assert!(path.is_empty(), "bulk entries must NOT fake a file path");
    }

    #[test]
    fn empty_path_must_not_reach_dir_open() {
        // The QEMU control's [3] entry: treating the empty path as a file
        // name gave Dir::open("") -> INVALID_PARAMETER.
        let data = option(&[&device()]);
        let (_, path) = parse_option_device_scope(&data).unwrap();
        assert_eq!(path.len(), 0, "bulk entry must not carry a file path");
    }

    #[test]
    fn fv_only_entry_parses_to_prefix() {
        // UiApp-style entry: one FvFile node, no media volume.
        let mut fv = vec![0x04, 0x06, 0x12, 0x00];
        fv.extend_from_slice(&[0x33; 14]); // GUID + type
        let data = option(&[&fv]);
        let (prefix, path) = parse_option_device_scope(&data).unwrap();
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

        // FILE node first: no device prefix.
        let data = option(&[&file(r"\EFI\BOOT\BOOTX64.EFI")]);
        assert!(parse_option_device_scope(&data).is_none());

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
    fn different_partition_signature_does_not_match() {
        let mut other_sig = SIG;
        other_sig[0] ^= 0x01;
        let vol = [[pci(0x01, 0x00), hd(2, other_sig, 0x02)].concat(), end()].concat();
        assert!(!volume_matches_prefix(&vol, &device()));
    }

    #[test]
    fn different_partition_number_does_not_match() {
        let vol = [[pci(0x01, 0x00), hd(3, SIG, 0x02)].concat(), end()].concat();
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
        let mut other_hd = SIG;
        other_hd[0] ^= 0x01;
        let other = [pci(0x01, 0x00), hd(2, other_hd, 0x02)].concat();
        assert_eq!(first_divergent_node(&device(), &other), 1);
        // Shorter sequence diverges at its own length.
        assert_eq!(first_divergent_node(&device(), &pci(0x01, 0x00)), 1);
    }
}
