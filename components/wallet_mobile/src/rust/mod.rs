mod asset;
mod transaction;
mod types;
mod util;

pub use asset::*;
pub use transaction::*;
pub use types::*;
pub use util::*;

use wasm_bindgen::prelude::*;

/// Constant defining the git commit hash and commit date of the commit this library was built
/// against.
const BUILD_ID: &str = concat!(env!("VERGEN_SHA_SHORT"), " ", env!("VERGEN_BUILD_DATE"));

/// Returns the git commit hash and commit date of the commit this library was built against.
#[wasm_bindgen]
pub fn build_id() -> String {
    BUILD_ID.to_string()
}
