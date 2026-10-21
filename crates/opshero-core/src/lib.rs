//! Pure, deterministic core of opshero.
//!
//! No I/O happens in this crate: it turns already-loaded data (evidence
//! records, a policy text, the subject tree id) into a verdict. Everything
//! that touches git, the filesystem or processes lives in the CLI crate.
//!
//! Design references: docs/rfc/0001..0004 and docs/adr/.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod canon;
pub mod journal;
pub mod policy;
pub mod record;
pub mod render;
pub mod shellcmd;
pub mod stats;
pub mod timefmt;
pub mod verify;
