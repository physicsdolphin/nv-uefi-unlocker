//! falcgen — position-independent fuc5 decoder and first-principles V67
//! payload generator. G0 lands the decoder (a Rust transcription of
//! envytools envydis/falcon.c tables + core-dis.c engine semantics) and
//! the ROP gadget scanner; G1/G2 add stack-geometry derivation and
//! payload synthesis (see docs/first-principles-payload-generator.md).

pub mod decode;
pub mod isa;
pub mod scan;
