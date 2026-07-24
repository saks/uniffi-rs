{#- Template for the error reader methods hash.
    Placed after all type definitions so Object error classes are available. -#}
  # Map error Class/Module objects to the stream reader method symbol.
  # The stream includes the local + all external mixins, so a single table
  # covers both local and external errors.
  ERROR_READER_METHODS = {
{% for e in ci.enum_definitions() %}
{%- if ci.is_name_used_as_error(e.name()) -%}
    {{ e.name()|class_name_rb }} => :read_{{ self::canonical_name(e.as_type().borrow()) }},
{% endif %}
{%- endfor -%}
{% for obj in ci.object_definitions() %}
{%- if ci.is_name_used_as_error(obj.name()) -%}
    {{ obj.name()|class_name_rb }} => :read_{{ self::canonical_name(obj.as_type().borrow()) }},
{% endif %}
{%- endfor -%}
{%- for type_ in ci.iter_external_types() -%}
{%- match type_ -%}
{%- when Type::Enum { name, .. } %}
{%- if ci.is_name_used_as_error(name) -%}
    ::{{ self.external_type_module(type_.module_path().unwrap()) }}::{{ name|class_name_rb }} => :read_{{ self::canonical_name(type_) }},
{%- endif -%}
{%- when Type::Object { name, .. } %}
{%- if ci.is_name_used_as_error(name) -%}
    ::{{ self.external_type_module(type_.module_path().unwrap()) }}::{{ name|class_name_rb }} => :read_{{ self::canonical_name(type_) }},
{%- endif -%}
{%- else -%}
{%- endmatch -%}
{%- endfor -%}
  }.freeze

  private_constant :ERROR_READER_METHODS
