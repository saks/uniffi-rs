{#-
// Template for external types from other crates.
// Adds a require statement for the external module's bindings,
// and generates an external error handler if the type is used as an error.
-#}
{%- match type_ %}
{%- when Type::Custom { name, module_path, .. } %}
{%- let ns = ci.namespace_for_module_path(module_path)? %}
{{ self.add_require(ns) }}
{%- match config.custom_types.get(name.as_str()) %}
{%- when Some(cfg) %}
{%- match cfg.imports %}
{%- when Some(imports) %}
# External custom type `{{ name }}`: importing configured dependencies.
{%- for import_name in imports %}
require '{{ import_name }}'
{%- endfor %}
{%- when None %}
{%- endmatch %}
{%- when None %}
{%- endmatch %}
{%- when Type::Enum { name, module_path, .. } %}
{%- let ns = ci.namespace_for_module_path(module_path)? %}
{{ self.add_require(ns) }}
{%- when Type::Object { name, module_path, .. } -%}
{%- let ns = ci.namespace_for_module_path(module_path)? %}
{{ self.add_require(ns) }}
{%- when Type::Record { module_path, .. } %}
{%- let ns = ci.namespace_for_module_path(module_path)? %}
{{ self.add_require(ns) }}
{%- when Type::CallbackInterface { module_path, .. } %}
{%- let ns = ci.namespace_for_module_path(module_path)? %}
{{ self.add_require(ns) }}
{%- else %}
{%- endmatch %}
