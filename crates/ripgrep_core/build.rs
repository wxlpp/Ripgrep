fn main() {
    uniffi::generate_scaffolding("./src/ripgrep_core.udl").ok();
    // We use proc-macro mode primarily; the UDL is optional fallback.
}
