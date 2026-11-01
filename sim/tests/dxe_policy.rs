#[path = "../../loader/src/dxe_policy.rs"]
mod dxe_policy;

#[test]
fn every_runtime_control_is_in_the_frozen_policy() {
    // New transaction controls must not silently escape authorization.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../loader/src");
    for file in [
        "tx.rs",
        "tx_fwsec.rs",
        "tx_booter.rs",
        "tx_detach.rs",
        "pci.rs",
        "runner.rs",
        "dxe.rs",
    ] {
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        for literal in source.split('"').skip(1).step_by(2) {
            if literal.starts_with("\\\\") && literal.ends_with(".txt") && !literal.contains(' ') {
                let path = literal.replace("\\\\", "\\");
                if ["\\tx-dxe-arm-install.txt", "\\tx-dxe-arm-remove.txt"].contains(&path.as_str())
                {
                    continue;
                }
                assert!(
                    dxe_policy::CONFIG_PATHS.contains(&path.as_str()),
                    "{file}: unpinned {path}"
                );
            }
        }
    }
}

#[test]
fn scrubber_and_optional_payload_integrity_use_actual_policy_code() {
    assert_eq!(dxe_policy::ATTEMPT_LIMIT, 2);
    use sha2::{Digest, Sha256};
    let bytes = b"scrubber";
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let manifest = format!("{digest}  \\scrubber_ad10x.bin\n");
    let check = |data| {
        dxe_policy::verify_assets(
            manifest.as_bytes(),
            [("\\scrubber_ad10x.bin", data)].into_iter(),
        )
    };
    assert!(check(Some(bytes.as_slice())).is_ok());
    assert!(check(Some(b"corrupt".as_slice())).is_err());
    assert!(check(None).is_err(), "expected asset removed");
    assert!(
        dxe_policy::verify_assets(b"", [("\\v67-tu10x-four-plm.bin", None)].into_iter()).is_ok(),
        "intentional replay allowed"
    );
    let duplicate = manifest.repeat(2);
    assert!(dxe_policy::verify_assets(
        duplicate.as_bytes(),
        [("\\scrubber_ad10x.bin", Some(bytes.as_slice()))].into_iter()
    )
    .is_err());
}

#[test]
fn removing_direct_payload_cannot_switch_to_family_or_replay() {
    use sha2::{Digest, Sha256};
    let direct = b"direct";
    let family = b"family";
    let hex = |bytes: &[u8]| -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    for family_present in [false, true] {
        let mut manifest = format!("{}  \\v67-direct-ss.bin\n", hex(direct));
        if family_present {
            manifest += &format!("{}  \\v67-tu10x-four-plm.bin\n", hex(family));
        }
        let family_bytes = family_present.then_some(family.as_slice());
        let check = |override_bytes| {
            dxe_policy::verify_assets(
                manifest.as_bytes(),
                [
                    ("\\v67-direct-ss.bin", override_bytes),
                    ("\\v67-tu10x-four-plm.bin", family_bytes),
                ]
                .into_iter(),
            )
        };
        assert!(check(Some(direct.as_slice())).is_ok());
        assert!(
            check(None).is_err(),
            "must refuse deletion, whether family fallback exists or not"
        );
    }
}
