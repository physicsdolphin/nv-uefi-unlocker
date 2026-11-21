//! Golden self-test: synth is a pure function, so the pinned payload
//! digests pin the preset outputs without the golden files entering git
//! (asset discipline: payloads/goldens/plaintexts never commit).
//! Pin sources: assets/src/manifest.rs (V67_*_PAYLOAD_SHA256) and the
//! gpu-unlock-bar1 eight-card artifact (docs; PR #95).

use falcgen::synth::{
    preset_bar1, preset_ss, sha256_hex, synth, synth_ad102, synth_ga102, Geometry,
    MARKER_AD102, MARKER_AD102_BAR1,
};

const TU102: &str = "b0ec90294d1de3c67e024631c0a11000f281470d76eed806fc24c66b7fd1ef41";
const TU11X: &str = "c79208ac3b9e49cb590dadceb84e7032055fbdfad27788d8da7cb6964a005750";
const GA102: &str = "2a0aa1f86d0b6afff5214d641d87d46dc98ef06ed908ff05d0ddbad5ffb7e197";
const AD102: &str = "44c36b2d6b3424e53e18f3e55db85c833655de071ce66ce42749588a4a7dc8cb";
/// gpu-unlock-bar1 eight-card 64G artifact (bar1-64g-report.md).
const AD102_BAR1_64G: &str =
    "cf3566d0887d3d31e8cb3611bf0a1b3fafcd9584ea7a73306f75f8d0eafb1284";

#[test]
fn ss_preset_reproduces_the_pinned_tables() {
    let got = sha256_hex(&synth(&preset_ss("tu102").unwrap(), &Geometry::tu102()));
    assert_eq!(got, TU102);
    let got = sha256_hex(&synth(&preset_ss("tu11x").unwrap(), &Geometry::tu11x()));
    assert_eq!(got, TU11X);
    let got = sha256_hex(&synth_ga102(&preset_ss("ga102").unwrap()));
    assert_eq!(got, GA102);
    let got = sha256_hex(&synth_ad102(&preset_ss("ad102").unwrap(), MARKER_AD102));
    assert_eq!(got, AD102);
}

#[test]
fn bar1_64g_preset_reproduces_the_bar1_artifact() {
    let (writes, proven) = preset_bar1(64).unwrap();
    assert!(proven);
    let got = sha256_hex(&synth_ad102(&writes, MARKER_AD102_BAR1));
    assert_eq!(got, AD102_BAR1_64G);
}
