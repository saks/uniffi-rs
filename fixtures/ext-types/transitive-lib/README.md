This crate consumes `sub-lib` only — it never names a `uniffi-one` type in its
own API. `SubLibType` still embeds `UniffiOneEnum` / trait / interface fields,
so Ruby (de)serialization of those nested values is a lexical call in
`sub-lib`'s mixin (`::UniffiOneNs::RustBuffer*Mixin.read/write_TypeUniffiOneEnum`).
Requiring `imported_types_sublib` still loads `uniffi_one_ns`.

`require` membership comes from `iter_external_types()`, not from
`[bindings.ruby.external_packages]`. Nested `UniffiOne*` fields are absent from
this crate's CI (`Type::Record` does not expose field types), so the generated
consumer must not `require 'uniffi_one_ns'` or `include ::UniffiOneNs::…`.
Library mode still auto-fills every peer crate into `external_packages` for
module-name lookup; that map is not an allow-list of direct externals.

The existing `ext-types/lib` fixture cannot catch a missing transitive call:
it uses `UniffiOne*` types directly, so its require set is a superset.
