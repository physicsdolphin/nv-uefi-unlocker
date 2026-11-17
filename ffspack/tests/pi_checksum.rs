//! Independent wire-format assertions: deliberately do not use packager
//! checksum offsets or verification helpers as the acceptance oracle.
use ffspack::{build_ffs, guid_bytes, NVUNLOCK_DXE_GUID};
#[test]
fn pi_header_and_payload_checksums_use_the_spec_offsets() {
    for state in [0xf8, 0x07] {
        let f = build_ffs(
            b"test PE bytes",
            &guid_bytes(NVUNLOCK_DXE_GUID).unwrap(),
            "test",
            state,
        )
        .unwrap();
        let header_sum = f[..24]
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 17 && *i != 23)
            .fold(0u8, |sum, (_, b)| sum.wrapping_add(*b));
        assert_eq!(header_sum, 0, "PI Header is byte 16, exclude File byte 17");
        let data_sum = f[24..].iter().fold(f[17], |sum, b| sum.wrapping_add(*b));
        assert_eq!(data_sum, 0, "PI File checksum is byte 17");
    }
}
