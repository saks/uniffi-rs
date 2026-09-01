This crate consumes `sub-lib` only — it never names a `uniffi-one` type in its
own API. `SubLibType` still embeds `UniffiOneEnum` / trait / interface fields,
so Ruby (de)serialization of those nested values must go through `sub-lib`'s
mixin, which has to include `uniffi-one`'s mixin.

Mixin / `require` membership comes from `iter_external_types()`, not from
`[bindings.ruby.external_packages]`. Nested `UniffiOne*` fields are absent from
this crate's CI (`Type::Record` does not expose field types), so the generated
consumer must not `require 'uniffi_one_ns'` or `include ::UniffiOneNs::…`.
Library mode still auto-fills every peer crate into `external_packages` for
module-name lookup; that map is not an allow-list of direct externals.

The existing `ext-types/lib` fixture cannot catch a missing transitive mixin:
it uses `UniffiOne*` types directly, so its include set is a superset.
