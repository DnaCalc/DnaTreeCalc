//! Shim: the tree workspace model moved into `dnacalc-host-core` (`tree::model`)
//! in S4.P2 so the RichTree session can live behind the Skin IR in host-core.
//! Re-exported here so the estate's `crate::model::…` call sites compile
//! unchanged. The types themselves are pure `serde`/`std` (no engine, no IR).

pub use dnacalc_host_core::tree::model::*;
