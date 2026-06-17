/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::Result;
use askama::Template;

use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

use crate::interface::{Enum, *};

const RESERVED_WORDS: &[&str] = &[
    "alias", "and", "BEGIN", "begin", "break", "case", "class", "def", "defined?", "do", "else",
    "elsif", "END", "end", "ensure", "false", "for", "if", "module", "next", "nil", "not", "or",
    "redo", "rescue", "retry", "return", "self", "super", "then", "true", "undef", "unless",
    "until", "when", "while", "yield", "__FILE__", "__LINE__",
];

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
        match self.external_packages.get(crate_name) {
            Some(name) => name.clone(),
            None => {
                let ns_name = namespace.unwrap_or(module_path);
                class_name_rb_inner(ns_name).unwrap_or_else(|_| ns_name.to_string())
            }
        }
    }
}

#[derive(Template)]
#[template(syntax = "rb", escape = "none", path = "wrapper.rb")]
pub struct RubyWrapper<'a> {
    config: Config,
    ci: &'a ComponentInterface,
    requires: RefCell<BTreeSet<String>>,
}
impl<'a> RubyWrapper<'a> {
    pub fn new(config: Config, ci: &'a ComponentInterface) -> Self {
        Self {
            config,
            ci,
            requires: RefCell::new(BTreeSet::new()),
        }
    }

    /// Add a `require` statement for an external module's binding file.
    /// Returns an empty string so it can be used inside an askama `{{ }}` block.
    pub fn add_require(&self, path: &str) -> &str {
        self.requires.borrow_mut().insert(path.to_owned());
        ""
    }

    /// Get the sorted, deduplicated list of require paths.
    pub fn requires(&self) -> Vec<String> {
        self.requires.borrow().iter().cloned().collect()
    }

    /// Resolve the Ruby module name for an external type's crate.
    /// Uses config.external_packages if configured, otherwise falls back to the namespace name.
    pub fn external_type_module(&self, module_path: &str) -> String {
        let namespace = self.ci.namespace_for_module_path(module_path).ok();
        self.config.external_package_name(module_path, namespace)
    }

    /// Generate the fully-qualified class reference for a named external type.
    /// E.g., `ExternalModule::ClassName`.
    pub fn external_class_name(&self, module_path: &str, name: &str) -> String {
        let module = self.external_type_module(module_path);
        let class_name = class_name_rb_inner(name).unwrap_or_else(|_| name.to_string());
        format!("{module}::{class_name}")
    }

    /// Returns true if the module_path comes from a different crate.
    pub fn is_external_module(&self, module_path: &str) -> bool {
        crate_name_from_module_path(module_path) != self.ci.crate_name()
    }

    pub fn initialization_fns(&self) -> Vec<String> {
        let extern_module_init_fns = self
            .ci
            .iter_external_types()
            .filter_map(|ty| ty.crate_name())
            .map(|crate_name| {
                let module = self.external_type_module(crate_name);
                let init_fn = format!("uniffi_ensure_{}_initialized", crate_name.to_snake_case());
                format!("{module}.{init_fn}() if defined?({module}.{init_fn})")
            })
            .collect::<BTreeSet<_>>();

        extern_module_init_fns.into_iter().collect()
    }

    /// Whether a given type is external (from another crate).
    pub fn is_external_type(&self, type_: &Type) -> bool {
        self.ci.is_external(type_)
    }
}

fn class_name_rb_inner(nm: &str) -> Result<String, askama::Error> {
    Ok(nm.to_string().to_upper_camel_case())
}

mod filters {
    use super::*;

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

    fn default_rb_inner(default: &DefaultValue) -> Result<String, askama::Error> {
        let DefaultValue::Literal(literal) = default else {
            unimplemented!("not supported.");
        };
        literal_rb_inner(literal)
    }

    fn literal_rb_inner(literal: &Literal) -> Result<String, askama::Error> {
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
            Literal::Some { inner } => default_rb_inner(inner)?,
            Literal::EmptySequence => "[]".into(),
            Literal::EmptyMap => "{}".into(),
            Literal::EmptySet => "Set.new".into(),
            Literal::Enum(v, type_) => match type_ {
                Type::Enum { name, .. } => {
                    format!("{}::{}", class_name_rb_inner(name)?, enum_name_rb_inner(v)?)
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
    fn type_zero_value_rb(ty: &Type) -> Result<String, askama::Error> {
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
            // Named types with no-arg constructors
            Type::Record { name, .. } | Type::Object { name, .. } => {
                format!("{}.new", class_name_rb_inner(name)?)
            }
            // Custom types delegate to their underlying builtin
            Type::Custom { builtin, .. } => type_zero_value_rb(builtin)?,
            _ => {
                return Err(askama::Error::Custom(
                    anyhow::anyhow!("No zero value for type {ty:?}").into(),
                ))
            }
        })
    }

    /// Render the Ruby default value for a field, handling both `Default` and `Literal` variants.
    #[askama::filter_fn]
    pub fn field_default_rb(
        field: &Field,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        match field.default_value() {
            Some(DefaultValue::Default) => {
                let ty = field.as_type();
                type_zero_value_rb(&ty)
            }
            Some(DefaultValue::Literal(lit)) => literal_rb_inner(lit),
            None => Err(askama::Error::Custom(
                anyhow::anyhow!("field_default_rb called on field with no default value").into(),
            )),
        }
    }

    /// Render the Ruby default value for a function/method argument.
    #[askama::filter_fn]
    pub fn arg_default_rb(arg: &Argument, _: &dyn askama::Values) -> Result<String, askama::Error> {
        match arg.default_value() {
            Some(DefaultValue::Default) => type_zero_value_rb(&arg.as_type()),
            Some(DefaultValue::Literal(lit)) => literal_rb_inner(lit),
            None => Err(askama::Error::Custom(
                anyhow::anyhow!("arg_default_rb called on arg with no default value").into(),
            )),
        }
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

    #[askama::filter_fn]
    pub fn coerce_rb<S1: AsRef<str>, S2: AsRef<str>>(
        nm: S1,
        _: &dyn askama::Values,
        ns: S2,
        type_: &Type,
        config: &Config,
    ) -> Result<String, askama::Error> {
        coerce_rb_inner(nm, ns, type_, &config.custom_types)
    }

    pub fn coerce_rb_inner<S1: AsRef<str>, S2: AsRef<str>>(
        nm: S1,
        ns: S2,
        type_: &Type,
        custom_types: &HashMap<String, CustomTypeConfig>,
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
                    coerce_rb_inner(nm, ns, t, custom_types)?
                )
            }
            Type::Sequence { inner_type: t } => {
                let coerce_code = coerce_rb_inner("v", ns, t, custom_types)?;
                if coerce_code == "v" {
                    nm.to_string()
                } else {
                    format!("{nm}.map {{ |v| {coerce_code} }}")
                }
            }
            Type::Set { inner_type: t } => {
                let coerce_code = coerce_rb_inner("v", ns, t, custom_types)?;
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
                let k_coerce_code = coerce_rb_inner("k", ns, kt, custom_types)?;
                let v_coerce_code = coerce_rb_inner("v", ns, vt, custom_types)?;

                if k_coerce_code == "k" && v_coerce_code == "v" {
                    nm.to_string()
                } else {
                    format!(
                        "{nm}.each.with_object({{}}) {{ |(k, v), res| res[{k_coerce_code}] = {v_coerce_code} }}"
                    )
                }
            }
            Type::Box { inner_type } => coerce_rb_inner(nm, ns, inner_type, custom_types)?,
            Type::Custom { name, builtin, .. } => {
                // For config-backed custom types, the user passes a custom-typed values;
                // skip builtin coercion (the lower expression handles conversion).
                if custom_types.contains_key(name) {
                    nm.to_string()
                } else {
                    coerce_rb_inner(nm, ns, builtin, custom_types)?
                }
            }
        })
    }

    #[askama::filter_fn]
    pub fn check_lower_rb(
        nm: impl AsRef<str>,
        _: &dyn askama::Values,
        type_: &Type,
        config: &Config,
        ci: &ComponentInterface,
    ) -> Result<String, askama::Error> {
        let module = if ci.is_external(type_) {
            Some(module_for_type(type_, &config.external_packages, ci)?)
        } else {
            None
        };
        check_lower_rb_inner(nm.as_ref(), type_, config, module.as_deref())
    }

    fn check_lower_rb_inner(
        nm: &str,
        type_: &Type,
        config: &Config,
        module: Option<&str>,
    ) -> Result<String, askama::Error> {
        let prefix = |s: &str| match module {
            Some(m) => format!("{m}::{s}"),
            None => s.to_string(),
        };
        Ok(match type_ {
            Type::Object { name, .. } => {
                format!(
                    "({}.uniffi_check_lower {nm})",
                    prefix(&class_name_rb_inner(name)?)
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
                    prefix(""),
                    canonical_name(type_)
                )
            }
            Type::Custom { name, .. } => {
                if let Some(cfg) = config.custom_types.get(name) {
                    if let Some(type_name) = &cfg.type_name {
                        format!(
                            "raise TypeError, \"Expected {type_name}, got {{#{nm}.class}}\" unless {nm}.is_a?({type_name})"
                        )
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        })
    }

    pub fn lower_rb_inner(
        nm: &str,
        type_: &Type,
        custom_types: &HashMap<String, CustomTypeConfig>,
    ) -> Result<String, askama::Error> {
        lower_rb_inner_dispatch(nm, type_, custom_types, None)
    }

    pub fn lower_rb_inner_dispatch(
        nm: &str,
        type_: &Type,
        custom_types: &HashMap<String, CustomTypeConfig>,
        module: Option<&str>,
    ) -> Result<String, askama::Error> {
        if let Type::Box { inner_type } = type_ {
            return lower_rb_inner_dispatch(nm, inner_type, custom_types, module);
        }
        if let Type::Custom { name, builtin, .. } = type_ {
            let nm = if let Some(cfg) = custom_types.get(name) {
                cfg.lower(nm)
            } else {
                nm.to_string()
            };
            return lower_rb_inner_dispatch(&nm, builtin, custom_types, module);
        }
        let prefix = |s: &str| match module {
            Some(m) => format!("{m}::{s}"),
            None => s.to_string(),
        };
        Ok(match type_ {
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
                    prefix(&class_name_rb_inner(name)?)
                )
            }
            Type::CallbackInterface { name, .. } => {
                format!(
                    "({}CallbackInterface{}FfiConverter.lower {})",
                    prefix(""),
                    class_name_rb_inner(name)?,
                    nm
                )
            }
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
                    prefix(""),
                    canonical_name(type_),
                    nm
                )
            }
            Type::Box { .. } | Type::Custom { .. } => unreachable!(),
        })
    }

    #[askama::filter_fn]
    pub fn lower_rb(
        nm: impl AsRef<str>,
        _: &dyn askama::Values,
        type_: &Type,
        config: &Config,
        ci: &ComponentInterface,
    ) -> Result<String, askama::Error> {
        let module = if ci.is_external(type_) {
            Some(module_for_type(type_, &config.external_packages, ci)?)
        } else {
            None
        };
        lower_rb_inner_dispatch(nm.as_ref(), type_, &config.custom_types, module.as_deref())
    }

    pub fn lift_rb_inner_dispatch(
        nm: &str,
        type_: &Type,
        custom_types: &HashMap<String, CustomTypeConfig>,
        module: Option<&str>,
    ) -> Result<String, askama::Error> {
        if let Type::Box { inner_type } = type_ {
            return lift_rb_inner_dispatch(nm, inner_type, custom_types, module);
        }
        if let Type::Custom { name, builtin, .. } = type_ {
            let lifted = lift_rb_inner_dispatch(nm, builtin, custom_types, module)?;
            return Ok(if let Some(cfg) = custom_types.get(name) {
                cfg.lift(&lifted)
            } else {
                lifted
            });
        }
        let prefix = |s: &str| match module {
            Some(m) => format!("{m}::{s}"),
            None => s.to_string(),
        };
        Ok(match type_ {
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
                format!("{}.uniffi_lift({nm})", prefix(&class_name_rb_inner(name)?))
            }
            Type::CallbackInterface { name, .. } => {
                format!(
                    "({}CallbackInterface{}FfiConverter.lift {nm})",
                    prefix(""),
                    class_name_rb_inner(name)?
                )
            }
            Type::Enum { .. } => match module {
                Some(m) => format!(
                    "({m}::RustBuffer.new.tap {{ |buf| buf[:capacity] = {nm}[:capacity]; buf[:len] = {nm}[:len]; buf[:data] = {nm}[:data] }}.consume_into_{})",
                    class_name_rb_inner(&canonical_name(type_))?
                ),
                None => format!(
                    "{nm}.consume_into_{}",
                    class_name_rb_inner(&canonical_name(type_))?
                ),
            },
            Type::Record { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Set { .. }
            | Type::Timestamp
            | Type::String
            | Type::Bytes
            | Type::Duration
            | Type::Map { .. } => match module {
                Some(m) => format!(
                    "({m}::RustBuffer.new.tap {{ |buf| buf[:capacity] = {nm}[:capacity]; buf[:len] = {nm}[:len]; buf[:data] = {nm}[:data] }}.consume_into_{})",
                    canonical_name(type_)
                ),
                None => format!("{nm}.consume_into_{}", canonical_name(type_)),
            },
            Type::Box { .. } | Type::Custom { .. } => unreachable!(),
        })
    }

    /// Return the Ruby expression that lifts a lowered value `nm` into the given type.
    #[askama::filter_fn]
    pub fn lift_rb(
        nm: &str,
        _: &dyn askama::Values,
        type_: &Type,
        config: &Config,
        ci: &ComponentInterface,
    ) -> Result<String, askama::Error> {
        let module = if ci.is_external(type_) {
            Some(module_for_type(type_, &config.external_packages, ci)?)
        } else {
            None
        };
        lift_rb_inner_dispatch(nm, type_, &config.custom_types, module.as_deref())
    }

    /// Resolve the Ruby module name for an external type.
    fn module_for_type(
        type_: &Type,
        external_packages: &HashMap<String, String>,
        ci: &ComponentInterface,
    ) -> Result<String, askama::Error> {
        let module_path = type_.module_path().ok_or_else(|| {
            askama::Error::Custom(anyhow::anyhow!("no module path for type {type_:?}").into())
        })?;
        let crate_name = crate_name_from_module_path(module_path);
        if let Some(package) = external_packages.get(crate_name) {
            return Ok(package.clone());
        }
        ci.namespace_for_module_path(module_path)
            .map(|ns| class_name_rb_inner(ns).unwrap_or_else(|_| ns.to_string()))
            .map_err(|e| askama::Error::Custom(e.into()))
    }

    /// Render the Ruby expression that lowers the `self` value of a trait method.
    /// For Object types, this is `(ClassName.uniffi_lower self)`.
    /// For Record/Enum types, this serializes `self` into a RustBuffer.
    #[askama::filter_fn]
    pub fn lower_method_self_rb(
        meth: &Method,
        _: &dyn askama::Values,
        config: &Config,
    ) -> Result<String, askama::Error> {
        let self_type = meth
            .self_type()
            .expect("Trait method must have a self type");

        lower_rb_inner("self", &self_type, &config.custom_types)
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
