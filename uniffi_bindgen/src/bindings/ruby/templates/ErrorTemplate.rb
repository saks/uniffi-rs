class RustCallStatus < FFI::Struct
  layout :code,    :int8,
         :error_buf, RustBuffer

  def code
    self[:code]
  end

  def error_buf
    self[:error_buf]
  end

  def to_s
    "RustCallStatus(code=#{self[:code]})"
  end
end

# These match the values from the uniffi::rustcalls module
CALL_SUCCESS = 0
CALL_ERROR = 1
CALL_PANIC = 2
{%- for e in ci.enum_definitions() %}
{% if ci.is_name_used_as_error(e.name()) %}
{% if e.is_flat() %}
class {{ e.name()|class_name_rb }}
    {%- for variant in e.variants() %}
    {{ variant.name()|class_name_rb }} = Class.new StandardError
    {%- endfor %}
{% else %}
module {{ e.name()|class_name_rb }}
  {%- for variant in e.variants() %}
  class {{ variant.name()|class_name_rb }} < StandardError
    {%- let named_fields = variant.has_fields() && !variant.fields()[0].name().is_empty() %}
    {%- if named_fields %}
    def initialize({% for field in variant.fields() %}{{ field.name()|var_name_rb }}:{% if !loop.last %}, {% endif %}{% endfor %})
        {% for field in variant.fields() %}
        @{{ field.name()|var_name_rb }} = {{ field.name()|var_name_rb }}
        {% endfor %}
        super()
    end
    {% else %}
    def initialize({% for field in variant.fields() %}v{{ loop.index }}{% if !loop.last %}, {% endif %}{% endfor %})
        {% if variant.has_fields() %}
        @values = [{% for field in variant.fields() %}v{{ loop.index }}{% if !loop.last %}, {% endif %}{% endfor %}]
        {% endif %}
        super()
    end
    {% endif %}
    {%- if variant.has_fields() %}
    {%- if named_fields %}

    attr_reader {% for field in variant.fields() %}:{{ field.name()|var_name_rb }}{% if !loop.last %}, {% endif %}{% endfor %}
    {%- else %}

    attr_reader :values

    def [](index)
        @values[index]
    end
    {%- endif %}
    {% endif %}

    def to_s
      {%- if named_fields %}
        "#{self.class.name}({% for field in variant.fields() %}{{ field.name()|var_name_rb }}=#{@{{ field.name()|var_name_rb }}.inspect}{% if !loop.last %}, {% endif %}{% endfor %})"

      {%- else %}
      {%- if variant.has_fields() %}
        "#{self.class.name}(#{@values.inspect})"
      {%- else %}
        "#{self.class.name}()"
      {%- endif %}
      {%- endif %}

    end
  end
  {%- endfor %}
{% endif %}
end
{% endif %}
{%- endfor %}

# Map error class names to the RustBuffer method name that reads them
ERROR_MODULE_TO_READER_METHOD = {
{% for e in ci.enum_definitions() %}
{%- if ci.is_name_used_as_error(e.name()) -%}
  '{{ e.name()|class_name_rb }}' => :read_{{ self::canonical_name(e.as_type().borrow()) }},
{% endif %}
{%- endfor -%}
{% for obj in ci.object_definitions() %}
{%- if ci.is_name_used_as_error(obj.name()) -%}
  '{{ obj.name()|class_name_rb }}' => :read_{{ self::canonical_name(obj.as_type().borrow()) }},
{% endif %}
{%- endfor -%}
}

# Map external error class names to lambdas that lift and raise the error
CONSUME_EXTERNAL_ERROR = {
{%- for type_ in ci.iter_external_types() -%}
{%- match type_ -%}
{%- when Type::Enum { name, .. } -%}
{%- if ci.is_name_used_as_error(name) -%}
  '{{ name|class_name_rb }}' => ->(rust_buffer) { raise {{ "rust_buffer"|lift_rb(type_, config, ci) }} },
{%- endif -%}
{%- when Type::Object { name, .. } -%}
{%- if ci.is_name_used_as_error(name) -%}
  '{{ name|class_name_rb }}' => ->(rust_buffer) {
    rust_buffer.consumeWithStream { |stream| raise stream.read_{{ self::canonical_name(type_) }} }
  },
{%- endif -%}
{%- else -%}
{%- endmatch -%}
{%- endfor -%}
}

private_constant :ERROR_MODULE_TO_READER_METHOD, :CONSUME_EXTERNAL_ERROR,
                 :CALL_SUCCESS, :CALL_ERROR, :CALL_PANIC, :RustCallStatus

def self.consume_buffer_into_error(error_class_name, external_module, rust_buffer)
  if external_module
    CONSUME_EXTERNAL_ERROR.fetch(error_class_name).call(rust_buffer)
    return
  end
  rust_buffer.consumeWithStream do |stream|
    reader_method = ERROR_MODULE_TO_READER_METHOD.fetch(error_class_name)
    return stream.send(reader_method)
  end
end

class InternalError < StandardError
end

def self.rust_call(fn_name, *args)
  # Call a rust function
  rust_call_with_error(nil, nil, fn_name, *args)
end

def self.rust_call_with_error(error_class_name, external_module, fn_name, *args)
  # Call a rust function and handle errors
  #
  # Use this when the rust function returns a Result<>.  error_class_name is the
  # error class name; external_module is the module name for external types.


  # Note: RustCallStatus.new zeroes out the struct, which is exactly what we
  # want to pass to Rust (code=0, error_buf=RustBuffer(len=0, capacity=0,
  # data=NULL))
  status = RustCallStatus.new
  args << status

  result = UniFFILib.public_send(fn_name, *args)

  case status.code
  when CALL_SUCCESS
    result
  when CALL_ERROR
    if error_class_name.nil?
      status.error_buf.free
      raise InternalError, "CALL_ERROR with no error_module set"
    else
      raise consume_buffer_into_error(error_class_name, external_module, status.error_buf)
    end
  when CALL_PANIC
    # When the rust code sees a panic, it tries to construct a RustBuffer
    # with the message.  But if that code panics, then it just sends back
    # an empty buffer.
    if status.error_buf.len > 0
      raise InternalError, {{ "status.error_buf"|lift_rb(&Type::String, config, ci) }}
    else
      raise InternalError, "Rust panic"
    end
  else
    raise InternalError, "Unknown call status: #{status.code}"
  end
end

private_class_method :consume_buffer_into_error
