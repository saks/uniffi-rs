This crate consumes `sub-lib` only — it never names a `uniffi-one` type in its
own API. `SubLibType` still embeds `UniffiOneEnum` / trait / interface fields,
so Ruby (de)serialization of those nested values must go through `sub-lib`'s
mixin, which has to include `uniffi-one`'s mixin.

The existing `ext-types/lib` fixture cannot catch a missing transitive mixin:
it uses `UniffiOne*` types directly, so its include set is a superset.
