//! End-to-end tests of the web tools against loopback stub servers.
//!
//! Nothing here touches the real network: Gemini, the grounding redirect
//! host, and the pages are all stubs on `127.0.0.1`. `live_web_search` is the
//! one exception and skips unless `GEMINI_API_KEY` and `SUPERMATH_LIVE_WEB`
//! are set.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod contract;
mod gemini;
mod live;
mod read;
mod redirect;
mod research;
mod search;
mod support;
