// Library surface for headless/integration tests and reuse.
// Keep this lean to avoid coupling to bin-only types in main.rs.
pub mod app_dirs;
pub mod celebration;
pub mod language;
pub mod runtime;
pub mod session;
pub mod stats;
pub mod thok;
pub mod time_series;
pub mod typing_policy;
pub mod util;

// Keep the old lang module name for compatibility in tests/examples
pub use language as lang;
