//! Integration tests, compiled into a single test binary to keep build times down.
#![expect(
    clippy::unwrap_used,
    reason = "test helpers should fail loudly on broken fixtures"
)]

mod cli;
mod common;
mod corpus;
mod docs;
mod fix;
