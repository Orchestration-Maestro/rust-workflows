//! `rust-gate rules` and `rules --check`: a repository's rule map, the
//! organization's golden rules adapted to it (C-001), in three pages under
//! `docs/standards`. `golden.rs` reads the golden rules this release carries,
//! `page.rs` reads back what a repository wrote, `render.rs` writes the pages
//! and `step.rs` holds the steps.

mod golden;
mod page;
mod render;
mod step;

pub(super) use step::STEPS;
