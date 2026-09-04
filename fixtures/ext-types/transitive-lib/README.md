This crate consumes `sub-lib` only — it never names a `uniffi-one` type in its
own API. `SubLibType` still embeds `UniffiOne*` fields, so round-tripping it
exercises types that appear only as nested fields of an external record.

A bindings generator that looks at this crate's interface will see `sub-lib`,
not `uniffi-one`. Nested field types of a record are not walked into the
consumer's type set (`Type::iter_nested_types`), so `uniffi-one` is absent
here even though values of those types show up at runtime. `sub-lib`'s
generated bindings are what actually (de)serialize them.

`ext-types/lib` cannot catch a missing transitive import: it uses `UniffiOne*`
types directly, so its external-crate set already includes `uniffi-one`.
