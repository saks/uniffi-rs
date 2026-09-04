use uniffi_sublib::SubLibType;

/// Roundtrip an external record whose fields come from a *further* crate
/// (`uniffi-one`) that this crate never names in its own API.
#[uniffi::export]
fn roundtrip_sub(s: SubLibType) -> SubLibType {
    s
}

/// Local record wrapping that external type, so nested mixin dispatch also
/// runs through this crate's own `read_TypeWrapper` / `write_TypeWrapper`.
#[derive(uniffi::Record)]
pub struct Wrapper {
    pub sub: SubLibType,
}

#[uniffi::export]
fn roundtrip_wrapper(w: Wrapper) -> Wrapper {
    w
}

uniffi::setup_scaffolding!("imported_types_transitive");
