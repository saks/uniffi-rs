/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::Result;
use askama::Template;

use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::{BTreeSet, HashMap};

use crate::interface::{Enum, *};

const RESERVED_WORDS: &[&str] = &[
    "alias", "and", "BEGIN", "begin", "break", "case", "class", "def", "defined?", "do", "else",
    "elsif", "END", "end", "ensure", "false", "for", "if", "module", "next", "nil", "not", "or",
    "redo", "rescue", "retry", "return", "self", "super", "then", "true", "undef", "unless",
    "until", "when", "while", "yield", "__FILE__", "__LINE__",
];

// Info for an external crate's mixin modules, used in templates.
pub struct ExternalMixin {
    pub module_name: String,
    pub require_path: String,
}

fn is_reserved_word(word: &str) -> bool {
    RESERVED_WORDS.contains(&word)
}

/// Extract the crate name from a module path (everything before the first `::`).
fn crate_name_from_module_path(module_path: &str) -> &str {
    module_path.split("::").next().unwrap_or(module_path)
}

/// Get the canonical, unique-within-this-component name for a type.
///
/// When generating helper code for foreign language bindings, it's sometimes useful to be
/// able to name a particular type in order to e.g. call a helper function that is specific
/// to that type. We support this by defining a naming convention where each type gets a
/// unique canonical name, constructed recursively from the names of its component types (if any).
pub fn canonical_name(t: &Type) -> String {
    match t {
        // Builtin primitive types, with plain old names.
        Type::Int8 => "i8".into(),
        Type::UInt8 => "u8".into(),
        Type::Int16 => "i16".into(),
        Type::UInt16 => "u16".into(),
        Type::Int32 => "i32".into(),
        Type::UInt32 => "u32".into(),
        Type::Int64 => "i64".into(),
        Type::UInt64 => "u64".into(),
        Type::Float32 => "f32".into(),
        Type::Float64 => "f64".into(),
        Type::String => "string".into(),
        Type::Bytes => "bytes".into(),
        Type::Boolean => "bool".into(),
        // API defined types.
        // Note that these all get unique names, and the parser ensures that the names do not
        // conflict with a builtin type. We add a prefix to the name to guard against pathological
        // cases like a record named `SequenceRecord` interfering with `sequence<Record>`.
        // However, types that support importing all end up with the same prefix of "Type", so
        // that the import handling code knows how to find the remote reference.
        Type::Object { name, .. } => format!("Type{name}"),
        Type::Enum { name, .. } => format!("Type{name}"),
        Type::Record { name, .. } => format!("Type{name}"),
        Type::CallbackInterface { name, .. } => format!("CallbackInterface{name}"),
        Type::Timestamp => "Timestamp".into(),
        Type::Duration => "Duration".into(),
        // Recursive types.
        // These add a prefix to the name of the underlying type.
        // The component API definition cannot give names to recursive types, so as long as the
        // prefixes we add here are all unique amongst themselves, then we have no chance of
        // acccidentally generating name collisions.
        Type::Optional { inner_type } => format!("Optional{}", canonical_name(inner_type)),
        Type::Sequence { inner_type } => format!("Sequence{}", canonical_name(inner_type)),
        Type::Set { inner_type } => format!("Set{}", canonical_name(inner_type)),
        Type::Map {
            key_type,
            value_type,
        } => format!(
            "Map{}{}",
            canonical_name(key_type).to_upper_camel_case(),
            canonical_name(value_type).to_upper_camel_case()
        ),
        Type::Custom { name, .. } => format!("Type{name}"),
        Type::Box { inner_type } => canonical_name(inner_type),
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTypeConfig {
    type_name: Option<String>,
    imports: Option<Vec<String>>,
    into_custom: String, // b/w compat alias for lift
    lift: String,
    from_custom: String, // b/w compat alias for lower
    lower: String,
}

impl CustomTypeConfig {
    /// Produce a Ruby expression that lifts a raw-builtin value `nm` into the custom type.
    fn lift(&self, name: &str) -> String {
        let converter = if self.lift.is_empty() {
            &self.into_custom
        } else {
            &self.lift
        };
        converter.replace("{}", name)
    }

    /// Produce a Ruby expression that lowers a value `nm` to its raw builtin.
    fn lower(&self, name: &str) -> String {
        let converter = if self.lower.is_empty() {
            &self.from_custom
        } else {
            &self.lower
        };
        converter.replace("{}", name)
    }

    /// True if this config actually specifies conversion expressions.
    pub fn has_conversion(&self) -> bool {
        !self.lift.is_empty() || !self.into_custom.is_empty()
    }
}

// Some config options for it the caller wants to customize the generated ruby.
// Note that this can only be used to control details of the ruby *that do not affect the underlying component*,
// since the details of the underlying component are entirely determined by the `ComponentInterface`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub(super) cdylib_name: Option<String>,
    cdylib_path: Option<String>,
    #[serde(default)]
    custom_types: HashMap<String, CustomTypeConfig>,
    #[serde(default)]
    pub(super) exclude: Vec<String>,
    #[serde(default)]
    pub(super) rename: toml::Table,
    #[serde(default)]
    pub(super) external_packages: HashMap<String, String>,
}

impl Config {
    pub fn cdylib_name(&self) -> String {
        self.cdylib_name
            .clone()
            .unwrap_or_else(|| "uniffi".to_string())
    }

    pub fn custom_cdylib_path(&self) -> bool {
        self.cdylib_path.is_some()
    }

    pub fn cdylib_path(&self) -> String {
        self.cdylib_path.clone().unwrap_or_default()
    }

    pub fn external_package_name(&self, module_path: &str, namespace: Option<&str>) -> String {
        let crate_name = crate_name_from_module_path(module_path);

        self.external_packages
            .get(crate_name)
            .cloned()
            .unwrap_or_else(|| {
                let ns_name = namespace.unwrap_or(module_path);
                class_name_rb_inner(ns_name).unwrap_or_else(|_| ns_name.to_string())
            })
    }
}

#[derive(Template)]
#[template(syntax = "rb", escape = "none", path = "wrapper.rb")]
pub struct RubyWrapper<'a> {
    config: Config,
    ci: &'a ComponentInterface,
}
impl<'a> RubyWrapper<'a> {
    pub fn new(config: Config, ci: &'a ComponentInterface) -> Self {
        Self { config, ci }
    }

    /// Resolve the Ruby module name for an external type's crate.
    /// Uses config.external_packages if configured, otherwise falls back to the namespace name.
    pub fn external_type_module(&self, module_path: &str) -> String {
        let namespace = self.ci.namespace_for_module_path(module_path).ok();
        self.config.external_package_name(module_path, namespace)
    }

    /// Returns true if the module_path comes from a different crate.
    pub fn is_external_module(&self, module_path: &str) -> bool {
        crate_name_from_module_path(module_path) != self.ci.crate_name()
    }

    /// Returns the reader symbol for a function's error type, or `"nil"`.
    /// Unwraps Custom types to find the inner Enum/Object, then uses `canonical_name`.
    pub fn error_reader_symbol(&self, func: &impl Callable) -> String {
        let error_type = match func.throws_type() {
            Some(Type::Custom { builtin, .. }) => builtin.as_ref(),
            Some(type_) => type_,
            None => return "nil".into(),
        };
        match error_type {
            Type::Enum { .. } | Type::Object { .. } => {
                format!(":read_{}", canonical_name(error_type))
            }
            _ => "nil".into(),
        }
    }

    /// Returns deduplicated list of external mixin info (module name + require path).
    /// Used by wrapper.rb for `require` and RustBufferBuilder/Stream for `include`.
    pub fn external_mixin_modules(&self) -> Vec<ExternalMixin> {
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();

        for typ in self.ci.iter_external_types() {
            if let Some(module_path) = typ.module_path() {
                let module_name = self.external_type_module(module_path);
                if seen.insert(module_name.clone()) {
                    let require_path = self
                        .ci
                        .namespace_for_module_path(module_path)
                        .unwrap_or(module_path)
                        .to_owned();

                    result.push(ExternalMixin {
                        module_name,
                        require_path,
                    });
                }
            }
        }

        result
    }

    /// Returns deduplicated require paths declared by external custom type configs.
    ///
    /// This keeps wrapper.rb simple by doing all type/config matching in Rust.
    pub fn external_custom_type_imports(&self) -> Vec<String> {
        let mut imports = BTreeSet::new();

        for typ in self.ci.iter_external_types() {
            if let Type::Custom { name, .. } = typ {
                if let Some(cfg) = self.config.custom_types.get(name) {
                    if let Some(ref extra_imports) = cfg.imports {
                        for import_name in extra_imports {
                            imports.insert(import_name.to_string());
                        }
                    }
                }
            }
        }

        imports.into_iter().collect()
    }

    /// Module prefix for lift/lower/check_lower of an external type, if any.
    ///
    /// Only Object and CallbackInterface need a foreign-module prefix (handle /
    /// converter lookup). RustBuffer-backed external types must go through the
    /// *local* `RustBuffer` / `RustBufferBuilder` bridges so alloc/reserve/free
    /// stay on this crate's cdylib, see the comments on those bridges.
    fn ffi_module_prefix(&self, type_: &Type) -> Option<String> {
        match type_ {
            Type::Box { inner_type } => self.ffi_module_prefix(inner_type),
            // Custom conversions are applied locally; recurse into the builtin.
            Type::Custom { builtin, .. } => self.ffi_module_prefix(builtin),
            Type::Object { module_path, .. } | Type::CallbackInterface { module_path, .. }
                if self.is_external_module(module_path) =>
            {
                Some(self.external_type_module(module_path))
            }
            _ => None,
        }
    }

    /// Defining crate module and builtin for an imported custom type.
    ///
    /// Used by `coerce_rb` to treat an imported custom type as already the
    /// foreign value (skip builtin coercion). Lift/lower/check_lower walk
    /// every `Type::Custom` node in dispatch and call that crate's
    /// `uniffi_{lift,lower,check_lower}_*` — including when the custom type
    /// is the builtin of another custom type (`LocalUrl` wrapping `Url`).
    fn external_custom<'b>(&self, type_: &'b Type) -> Option<(String, &'b Type)> {
        match type_ {
            Type::Box { inner_type } => self.external_custom(inner_type),
            Type::Custom {
                module_path,
                builtin,
                ..
            } if self.is_external_module(module_path) => {
                Some((self.external_type_module(module_path), builtin.as_ref()))
            }
            _ => None,
        }
    }

    pub(crate) fn is_external_custom(&self, type_: &Type) -> bool {
        self.external_custom(type_).is_some()
    }

    /// Ruby module that owns `uniffi_lift_*` / `uniffi_lower_*` for a custom type.
    ///
    /// External types use the defining crate's module; local types use this
    /// crate's namespace so the call is valid from instance methods as well
    /// as `def self.` functions.
    pub(crate) fn custom_owner_module(&self, module_path: &str) -> String {
        if self.is_external_module(module_path) {
            self.external_type_module(module_path)
        } else {
            class_name_rb_inner(self.ci.namespace()).expect("namespace class name")
        }
    }

    pub fn lift_rb(&self, nm: &str, type_: &Type) -> String {
        let module = self.ffi_module_prefix(type_);
        filters::lift_rb_inner_dispatch(nm, type_, module.as_deref(), self)
            .expect("lift_rb_inner_dispatch failed")
    }

    pub fn lower_rb(&self, nm: impl AsRef<str>, type_: &Type) -> String {
        let module = self.ffi_module_prefix(type_);
        filters::lower_rb_inner_dispatch(nm.as_ref(), type_, module.as_deref(), self)
            .expect("lower_rb_inner_dispatch failed")
    }

    pub fn check_lower_rb(&self, nm: impl AsRef<str>, type_: &Type) -> String {
        let module = self.ffi_module_prefix(type_);
        filters::check_lower_rb_inner(nm.as_ref(), type_, module.as_deref(), self)
            .expect("check_lower_rb_inner failed")
    }

    pub fn coerce_rb(&self, nm: impl AsRef<str>, type_: &Type) -> String {
        let ns = class_name_rb_inner(self.ci.namespace()).expect("namespace class name");
        filters::coerce_rb_inner(nm, ns, type_, &self.config.custom_types, self)
            .expect("coerce_rb failed")
    }

    pub fn field_default_rb(&self, field: &Field) -> String {
        match field.default_value() {
            Some(default) => filters::default_rb_inner(default, &field.as_type(), self)
                .expect("field_default_rb failed"),
            None => panic!("field_default_rb called on field with no default value"),
        }
    }

    pub fn arg_default_rb(&self, arg: &Argument) -> String {
        match arg.default_value() {
            Some(default) => filters::default_rb_inner(default, &arg.as_type(), self)
                .expect("arg_default_rb failed"),
            None => panic!("arg_default_rb called on arg with no default value"),
        }
    }

    /// Module prefix for a Ruby *class name* of this type, if it lives in another crate.
    ///
    /// Unlike `ffi_module_prefix`, this applies to records and enums as well: defaults
    /// construct Ruby objects by class name and do not go through the local RustBuffer.
    pub(crate) fn type_class_module(&self, type_: &Type) -> Option<String> {
        match type_ {
            Type::Box { inner_type } => self.type_class_module(inner_type),
            Type::Custom { builtin, .. } => self.type_class_module(builtin),
            Type::Record { module_path, .. }
            | Type::Object { module_path, .. }
            | Type::Enum { module_path, .. }
                if self.is_external_module(module_path) =>
            {
                Some(self.external_type_module(module_path))
            }
            _ => None,
        }
    }
}

fn class_name_rb_inner(nm: &str) -> Result<String, askama::Error> {
    Ok(nm.to_string().to_upper_camel_case())
}

mod filters {
    use super::*;

    /// Qualify `name` with an optional external module path, e.g. `qualify("Foo", Some("Mod"))`
    /// yields `"Mod::Foo"`; with `None` it yields `"Foo"` unchanged. This is the single source
    /// of truth for prefixing names with their owning module across the lift/lower/check filters.
    fn qualify(name: &str, module: Option<&str>) -> String {
        match module {
            Some(m) => format!("{m}::{name}"),
            None => name.to_string(),
        }
    }

    #[askama::filter_fn]
    pub fn type_ffi(type_: &FfiType, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(match type_ {
            FfiType::Int8 => ":int8".to_string(),
            FfiType::UInt8 => ":uint8".to_string(),
            FfiType::Int16 => ":int16".to_string(),
            FfiType::UInt16 => ":uint16".to_string(),
            FfiType::Int32 => ":int32".to_string(),
            FfiType::UInt32 => ":uint32".to_string(),
            FfiType::Int64 => ":int64".to_string(),
            FfiType::UInt64 => ":uint64".to_string(),
            FfiType::Float32 => ":float".to_string(),
            FfiType::Float64 => ":double".to_string(),
            FfiType::Handle => ":uint64".to_string(),
            FfiType::RustBuffer(_) => "RustBuffer.by_value".to_string(),
            FfiType::RustCallStatus => "RustCallStatus".to_string(),
            FfiType::ForeignBytes => "ForeignBytes".to_string(),
            FfiType::Callback(name) => format!(":{name}"),
            FfiType::Reference(inner) | FfiType::MutReference(inner) => match inner.as_ref() {
                FfiType::Struct(name) => format!("{name}.by_ref"),
                _ => ":pointer".to_string(),
            },
            FfiType::VoidPointer => ":pointer".to_string(),
            FfiType::Struct(name) => format!("{name}.by_value"),
        })
    }

    /// Generate the Ruby FFI::Pointer write method name for writing a lowered return value.
    /// For RustBuffer returns, return "rustbuffer" as a sentinel - template handles it specially.
    #[askama::filter_fn]
    pub fn ffi_write_return_rb(
        return_type: &Type,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        let ffi_type = FfiType::from(return_type);

        Ok(match &ffi_type {
            FfiType::Int8 => "write_int8".to_string(),
            FfiType::UInt8 => "write_uint8".to_string(),
            FfiType::Int16 => "write_int16".to_string(),
            FfiType::UInt16 => "write_uint16".to_string(),
            FfiType::Int32 => "write_int32".to_string(),
            FfiType::UInt32 => "write_uint32".to_string(),
            FfiType::Int64 => "write_int64".to_string(),
            FfiType::UInt64 => "write_uint64".to_string(),
            FfiType::Float32 => "write_float".to_string(),
            FfiType::Float64 => "write_double".to_string(),
            FfiType::Handle => "write_uint64".to_string(),
            FfiType::RustBuffer(_) => "rustbuffer".to_string(),
            _ => panic!("Unsupported FFI return type for callback: {ffi_type:?}"),
        })
    }

    /// Return the Ruby default value for an FFI return type (used in async error callbacks).
    #[askama::filter_fn]
    pub fn ffi_default_value_rb(
        return_type: &Type,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        let ffi_type = FfiType::from(return_type);
        Ok(match &ffi_type {
            FfiType::Int8
            | FfiType::UInt8
            | FfiType::Int16
            | FfiType::UInt16
            | FfiType::Int32
            | FfiType::UInt32
            | FfiType::Int64
            | FfiType::UInt64
            | FfiType::Handle => "0".to_string(),
            FfiType::Float32 | FfiType::Float64 => "0.0".to_string(),
            FfiType::RustBuffer(_) => "RustBuffer.new".to_string(),
            _ => panic!("Unsupported FFI return type for callback: {ffi_type:?}"),
        })
    }

    /// Return the ForeignFutureResult struct name for a method's return type.
    #[askama::filter_fn]
    pub fn foreign_future_result_rb(
        method: &Method,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        Ok(method.foreign_future_ffi_result_struct().name().to_string())
    }

    pub(super) fn default_rb_inner(
        default: &DefaultValue,
        ty: &Type,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        match default {
            DefaultValue::Literal(lit) => literal_rb_inner(lit, ty, wrapper),
            DefaultValue::Default => type_zero_value_rb(ty, wrapper),
        }
    }

    fn literal_rb_inner(
        literal: &Literal,
        ty: &Type,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        Ok(match literal {
            Literal::Boolean(v) => {
                if *v {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            // use the double-quote form to match with the other languages, and quote escapes.
            Literal::String(s) => format!("\"{s}\""),
            Literal::None => "nil".into(),
            Literal::Some { inner } => {
                let inner_ty = match ty {
                    Type::Optional { inner_type } => inner_type.as_ref(),
                    // Peel Custom wrappers — the metadata construction already validated
                    // that the builtin is Optional; match type_zero_value_rb's convention.
                    Type::Custom { builtin, .. } => match builtin.as_ref() {
                        Type::Optional { inner_type } => inner_type.as_ref(),
                        other => {
                            return Err(askama::Error::Custom(
                                anyhow::anyhow!(
                                    "Expected Optional type for Some literal, got {other:?}"
                                )
                                .into(),
                            ));
                        }
                    },
                    _ => {
                        return Err(askama::Error::Custom(
                            anyhow::anyhow!("Expected Optional type for Some literal, got {ty:?}")
                                .into(),
                        ));
                    }
                };
                default_rb_inner(inner, inner_ty, wrapper)?
            }
            Literal::EmptySequence => "[]".into(),
            Literal::EmptyMap => "{}".into(),
            Literal::EmptySet => "Set.new".into(),
            Literal::Enum(v, type_) => match type_ {
                Type::Enum { name, .. } => {
                    format!(
                        "{}::{}",
                        qualify(
                            &class_name_rb_inner(name)?,
                            wrapper.type_class_module(type_).as_deref()
                        ),
                        enum_name_rb_inner(v)?
                    )
                }
                _ => panic!("Unexpected type in enum literal: {type_:?}"),
            },
            // https://docs.ruby-lang.org/en/2.0.0/syntax/literals_rdoc.html
            Literal::Int(i, radix, _) => match radix {
                Radix::Octal => format!("0o{i:o}"),
                Radix::Decimal => format!("{i}"),
                Radix::Hexadecimal => format!("{i:#x}"),
            },
            Literal::UInt(i, radix, _) => match radix {
                Radix::Octal => format!("0o{i:o}"),
                Radix::Decimal => format!("{i}"),
                Radix::Hexadecimal => format!("{i:#x}"),
            },
            Literal::Float(string, _type_) => string.clone(),
        })
    }

    /// Return the Ruby zero/default value for a type (used for `#[uniffi::default]`).
    fn type_zero_value_rb(ty: &Type, wrapper: &RubyWrapper<'_>) -> Result<String, askama::Error> {
        Ok(match ty {
            Type::Int8
            | Type::UInt8
            | Type::Int16
            | Type::UInt16
            | Type::Int32
            | Type::UInt32
            | Type::Int64
            | Type::UInt64 => "0".to_string(),
            Type::Float32 | Type::Float64 => "0.0".to_string(),
            Type::Boolean => "false".to_string(),
            Type::String => "\"\"".to_string(),
            Type::Optional { .. } => "nil".to_string(),
            Type::Sequence { .. } => "[]".to_string(),
            Type::Bytes => "\"\".b".to_string(),
            Type::Map { .. } => "{}".to_string(),
            Type::Set { .. } => "Set.new".to_string(),
            // Named types with no-arg constructors. Qualify external crates so
            // Ruby does not look the class up inside the consumer module.
            Type::Record { name, .. } | Type::Object { name, .. } => {
                format!(
                    "{}.new",
                    qualify(
                        &class_name_rb_inner(name)?,
                        wrapper.type_class_module(ty).as_deref()
                    )
                )
            }
            // Custom types delegate to their underlying builtin
            Type::Custom { builtin, .. } => type_zero_value_rb(builtin, wrapper)?,
            _ => {
                return Err(askama::Error::Custom(
                    anyhow::anyhow!("No zero value for type {ty:?}").into(),
                ))
            }
        })
    }

    #[askama::filter_fn]
    pub fn class_name_rb(nm: &str, _: &dyn askama::Values) -> Result<String, askama::Error> {
        class_name_rb_inner(nm)
    }

    #[askama::filter_fn]
    pub fn fn_name_rb(nm: &str, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(nm.to_string().to_snake_case())
    }

    #[askama::filter_fn]
    pub fn var_name_rb(nm: &str, _: &dyn askama::Values) -> Result<String, askama::Error> {
        let snake = nm.to_string().to_snake_case();
        let prefix = if is_reserved_word(&snake) { "_" } else { "" };

        Ok(format!("{prefix}{snake}"))
    }

    #[askama::filter_fn]
    pub fn enum_name_rb(nm: &str, _: &dyn askama::Values) -> Result<String, askama::Error> {
        enum_name_rb_inner(nm)
    }

    pub fn enum_name_rb_inner(nm: &str) -> Result<String, askama::Error> {
        Ok(nm.to_string().to_shouty_snake_case())
    }

    pub fn coerce_rb_inner<S1: AsRef<str>, S2: AsRef<str>>(
        nm: S1,
        ns: S2,
        type_: &Type,
        custom_types: &HashMap<String, CustomTypeConfig>,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        let nm = nm.as_ref();
        let ns = ns.as_ref();
        Ok(match type_ {
            Type::Int8 => format!("::{ns}::uniffi_in_range({nm}, \"i8\", -2**7, 2**7)"),
            Type::Int16 => format!("::{ns}::uniffi_in_range({nm}, \"i16\", -2**15, 2**15)"),
            Type::Int32 => format!("::{ns}::uniffi_in_range({nm}, \"i32\", -2**31, 2**31)"),
            Type::Int64 => format!("::{ns}::uniffi_in_range({nm}, \"i64\", -2**63, 2**63)"),
            Type::UInt8 => format!("::{ns}::uniffi_in_range({nm}, \"u8\", 0, 2**8)"),
            Type::UInt16 => format!("::{ns}::uniffi_in_range({nm}, \"u16\", 0, 2**16)"),
            Type::UInt32 => format!("::{ns}::uniffi_in_range({nm}, \"u32\", 0, 2**32)"),
            Type::UInt64 => format!("::{ns}::uniffi_in_range({nm}, \"u64\", 0, 2**64)"),
            Type::Float32
            | Type::Float64
            | Type::Object { .. }
            | Type::Enum { .. }
            | Type::Record { .. }
            | Type::Timestamp
            | Type::Duration
            | Type::CallbackInterface { .. } => nm.to_string(),
            Type::Boolean => format!("{nm} ? true : false"),
            Type::String => format!("::{ns}::uniffi_utf8({nm})"),
            Type::Bytes => format!("::{ns}::uniffi_bytes({nm})"),
            Type::Optional { inner_type: t } => {
                format!(
                    "({nm} ? {} : nil)",
                    coerce_rb_inner(nm, ns, t, custom_types, wrapper)?
                )
            }
            Type::Sequence { inner_type: t } => {
                let coerce_code = coerce_rb_inner("v", ns, t, custom_types, wrapper)?;
                if coerce_code == "v" {
                    nm.to_string()
                } else {
                    format!("{nm}.map {{ |v| {coerce_code} }}")
                }
            }
            Type::Set { inner_type: t } => {
                let coerce_code = coerce_rb_inner("v", ns, t, custom_types, wrapper)?;
                if coerce_code == "v" {
                    nm.to_string()
                } else {
                    format!("{nm}.map {{ |v| {coerce_code} }}.to_set")
                }
            }
            Type::Map {
                key_type: kt,
                value_type: vt,
            } => {
                let k_coerce_code = coerce_rb_inner("k", ns, kt, custom_types, wrapper)?;
                let v_coerce_code = coerce_rb_inner("v", ns, vt, custom_types, wrapper)?;

                if k_coerce_code == "k" && v_coerce_code == "v" {
                    nm.to_string()
                } else {
                    format!(
                        "{nm}.each.with_object({{}}) {{ |(k, v), res| res[{k_coerce_code}] = {v_coerce_code} }}"
                    )
                }
            }
            Type::Box { inner_type } => coerce_rb_inner(nm, ns, inner_type, custom_types, wrapper)?,
            Type::Custom { name, builtin, .. } => {
                // Config-backed or imported buffer-backed custom types are
                // already the foreign value; skip builtin coercion.
                if custom_types.contains_key(name) || wrapper.is_external_custom(type_) {
                    nm.to_string()
                } else {
                    coerce_rb_inner(nm, ns, builtin, custom_types, wrapper)?
                }
            }
        })
    }

    pub(super) fn check_lower_rb_inner(
        nm: &str,
        type_: &Type,
        module: Option<&str>,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        Ok(match type_ {
            Type::Object { name, .. } => {
                format!(
                    "({}.uniffi_check_lower {nm})",
                    qualify(&class_name_rb_inner(name)?, module)
                )
            }
            Type::Enum { .. }
            | Type::Record { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Set { .. }
            | Type::Map { .. } => {
                format!(
                    "{}RustBuffer.check_lower_{}({nm})",
                    qualify("", module),
                    canonical_name(type_)
                )
            }
            Type::Box { inner_type } => check_lower_rb_inner(nm, inner_type, module, wrapper)?,
            Type::Custom {
                name,
                builtin,
                module_path,
                ..
            } => {
                // External types always use the defining crate's checker.
                // Local types with a `type_name` use this crate's checker.
                // Identity local newtypes recurse so a wrapper like
                // `LocalUrl = Url` still checks `URI`.
                let has_local_type_name = wrapper
                    .config
                    .custom_types
                    .get(name)
                    .and_then(|cfg| cfg.type_name.as_ref())
                    .is_some();
                if wrapper.is_external_module(module_path) || has_local_type_name {
                    format!(
                        "{}.uniffi_check_lower_{}({nm})",
                        wrapper.custom_owner_module(module_path),
                        canonical_name(type_),
                    )
                } else {
                    check_lower_rb_inner(nm, builtin, module, wrapper)?
                }
            }
            _ => String::new(),
        })
    }

    pub fn lower_rb_inner_dispatch(
        nm: &str,
        type_: &Type,
        module: Option<&str>,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        Ok(match type_ {
            // Named-handle types that recurse without touching a RustBuffer.
            Type::Box { inner_type } => lower_rb_inner_dispatch(nm, inner_type, module, wrapper)?,
            Type::Custom {
                builtin,
                module_path,
                ..
            } => {
                // Convert via the owning module, then lower the builtin. Do not
                // also apply consumer `custom_types` — that lives in
                // `uniffi_lower_*` (CustomTypeTemplate.rb).
                let converted = format!(
                    "{}.uniffi_lower_{}({nm})",
                    wrapper.custom_owner_module(module_path),
                    canonical_name(type_),
                );
                lower_rb_inner_dispatch(&converted, builtin, module, wrapper)?
            }
            // Builtin primitives passed through by value.
            Type::Int8
            | Type::UInt8
            | Type::Int16
            | Type::UInt16
            | Type::Int32
            | Type::UInt32
            | Type::Int64
            | Type::UInt64
            | Type::Float32
            | Type::Float64 => nm.to_string(),
            Type::Boolean => format!("({nm} ? 1 : 0)"),
            Type::Object { name, .. } => {
                format!(
                    "({}.uniffi_lower {nm})",
                    qualify(&class_name_rb_inner(name)?, module)
                )
            }
            Type::CallbackInterface { name, .. } => {
                format!(
                    "({}CallbackInterface{}FfiConverter.lower {})",
                    qualify("", module),
                    class_name_rb_inner(name)?,
                    nm
                )
            }
            // Types serialized through a RustBuffer.
            Type::Enum { .. }
            | Type::Record { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Set { .. }
            | Type::Timestamp
            | Type::String
            | Type::Bytes
            | Type::Duration
            | Type::Map { .. } => {
                format!(
                    "{}RustBuffer.alloc_from_{}({})",
                    qualify("", module),
                    canonical_name(type_),
                    nm
                )
            }
        })
    }

    pub fn lift_rb_inner_dispatch(
        nm: &str,
        type_: &Type,
        module: Option<&str>,
        wrapper: &RubyWrapper<'_>,
    ) -> Result<String, askama::Error> {
        Ok(match type_ {
            // Named-handle types that recurse without touching a RustBuffer.
            Type::Box { inner_type } => lift_rb_inner_dispatch(nm, inner_type, module, wrapper)?,
            Type::Custom {
                builtin,
                module_path,
                ..
            } => {
                // Lift the builtin, then convert via the owning module. Do not
                // also apply consumer `custom_types` — that lives in
                // `uniffi_lift_*` (CustomTypeTemplate.rb).
                let lifted = lift_rb_inner_dispatch(nm, builtin, module, wrapper)?;
                format!(
                    "{}.uniffi_lift_{}({lifted})",
                    wrapper.custom_owner_module(module_path),
                    canonical_name(type_),
                )
            }
            // Builtin primitives passed through by value.
            Type::Int8
            | Type::UInt8
            | Type::Int16
            | Type::UInt16
            | Type::Int32
            | Type::UInt32
            | Type::Int64
            | Type::UInt64 => format!("{nm}.to_i"),
            Type::Float32 | Type::Float64 => format!("{nm}.to_f"),
            Type::Boolean => format!("1 == {nm}"),
            Type::Object { name, .. } => {
                format!(
                    "{}.uniffi_lift({nm})",
                    qualify(&class_name_rb_inner(name)?, module)
                )
            }
            Type::CallbackInterface { name, .. } => {
                format!(
                    "({}CallbackInterface{}FfiConverter.lift {nm})",
                    qualify("", module),
                    class_name_rb_inner(name)?
                )
            }
            Type::Enum { .. } => {
                format!(
                    "{nm}.consume_into_{}",
                    class_name_rb_inner(&canonical_name(type_))?
                )
            }
            // Types deserialized from a RustBuffer.
            Type::Record { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Set { .. }
            | Type::Timestamp
            | Type::String
            | Type::Bytes
            | Type::Duration
            | Type::Map { .. } => {
                format!("{nm}.consume_into_{}", canonical_name(type_))
            }
        })
    }

    /// Render the Ruby expression that lowers the `self` value of a trait method.
    /// For Object types, this is `(ClassName.uniffi_lower self)`.
    /// For Record/Enum types, this serializes `self` into a RustBuffer.
    #[askama::filter_fn]
    pub fn lower_method_self_rb(
        meth: &Method,
        _: &dyn askama::Values,
        wrapper: &RubyWrapper<'filter>,
    ) -> Result<String, askama::Error> {
        let self_type = meth
            .self_type()
            .expect("Trait method must have a self type");

        Ok(wrapper.lower_rb("self", &self_type))
    }

    /// Render a Ruby integer literal for the discriminant of the variant at `index` in enum `e`.
    #[askama::filter_fn]
    pub fn variant_discr_literal(
        e: &Enum,
        _: &dyn askama::Values,
        index: &usize,
    ) -> Result<String, askama::Error> {
        let literal = e
            .variant_discr(*index)
            .map_err(|err| askama::Error::Custom(err.into()))?;

        match literal {
            Literal::UInt(v, _, _) => Ok(v.to_string()),
            Literal::Int(v, _, _) => Ok(v.to_string()),
            _ => Err(askama::Error::Custom(
                anyhow::anyhow!("Only integer discriminants are supported").into(),
            )),
        }
    }
}

#[cfg(test)]
mod test_type {
    use super::*;

    #[test]
    fn test_canonical_names() {
        // Non-exhaustive, but gives a bit of a flavour of what we want.
        assert_eq!(canonical_name(&Type::UInt8), "u8");
        assert_eq!(canonical_name(&Type::String), "string");
        assert_eq!(canonical_name(&Type::Bytes), "bytes");
        assert_eq!(
            canonical_name(&Type::Optional {
                inner_type: Box::new(Type::Sequence {
                    inner_type: Box::new(Type::Object {
                        module_path: "anything".to_string(),
                        name: "Example".into(),
                        imp: ObjectImpl::Struct,
                    })
                })
            }),
            "OptionalSequenceTypeExample"
        );

        let map = Type::Map {
            key_type: Box::new(Type::UInt32),
            value_type: Box::new(Type::UInt32),
        };
        assert_eq!(canonical_name(&map), "MapU32U32");
        assert_eq!(
            canonical_name(&Type::Enum {
                module_path: "foo".to_string(),
                name: "HTMLError".to_string()
            }),
            "TypeHTMLError"
        );
    }

    #[test]
    fn test_class_name() {
        assert_eq!(class_name_rb_inner("Example").unwrap(), "Example");
    }
}

#[cfg(test)]
mod tests;
