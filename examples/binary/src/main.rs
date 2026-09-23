//! Binary consumer fixture that prints a checked arithmetic result.
//!
//! It carries one crates.io dependency, `anyhow`, so the dependency policy, the
//! SBOMs and the embedded dependency list describe something real.

#![forbid(unsafe_code)]

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let total = bounded_sum::checked_sum(20, 22).context("the sum overflows u32")?;
    println!("{total}");
    Ok(())
}
