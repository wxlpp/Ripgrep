//! ripgrep_core — Swift-facing wrapper around ripgrep's reusable crates.
//!
//! Public surface is generated via UniFFI; see `lib.rs` `uniffi::setup_scaffolding!()`.

mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_compiles() {
        assert_eq!(2 + 2, 4);
    }
}

#[cfg(test)]
mod error_tests {
    use super::error::RipgrepError;

    #[test]
    fn error_messages_render() {
        let e = RipgrepError::InvalidPattern("[".into());
        assert!(format!("{e}").contains("invalid regex"));
        assert!(format!("{e}").contains("["));
    }
}
