
# Helper for structured reading of values from a RustBuffer.
class RustBufferStream

  def initialize(rbuf)
    @rbuf = rbuf
    @offset = 0
  end

  def remaining
    @rbuf.len - @offset
  end

  def read(size)
    raise InternalError, 'read past end of rust buffer' if @offset + size > @rbuf.len

    data = @rbuf.data.get_bytes @offset, size

    @offset += size

    data
  end

  {%- for typ in ci.iter_types() -%}
  {%- let canonical_type_name = typ.canonical_name()|class_name_rb -%}
  {%- match typ -%}

  {% when Type::Int8 -%}

  def readI8
    unpack_from 1, 'c'
  end

  {% when Type::UInt8 -%}

  def readU8
    unpack_from 1, 'c'
  end

  {% when Type::Int16 -%}

  def readI16
    unpack_from 2, 's>'
  end

  {% when Type::UInt16 -%}

  def readU16
    unpack_from 1, 'S>'
  end

  {% when Type::Int32 -%}

  def readI32
    unpack_from 4, 'l>'
  end

  {% when Type::UInt32 -%}

  def readU32
    unpack_from 4, 'L>'
  end

  {% when Type::Int64 -%}

  def readI64
    unpack_from 8, 'q>'
  end

  {% when Type::UInt64 -%}

  def readU64
    unpack_from 8, 'Q>'
  end

  {% when Type::Float32 -%}

  def readF32
    unpack_from 4, 'g'
  end

  {% when Type::Float64 -%}

  def readF64
    unpack_from 8, 'G'
  end

  {% when Type::Boolean -%}

  def readBool
    v = unpack_from 1, 'c'

    return false if v == 0
    return true if v == 1

    raise InternalError, 'Unexpected byte for Boolean type'
  end

  {% when Type::String -%}

  def readString
    size = unpack_from 4, 'l>'

    raise InternalError, 'Unexpected negative string length' if size.negative?

    read(size).force_encoding(Encoding::UTF_8)
  end

  {% when Type::Object with (object_name) -%}
  # The Object type {{ object_name }}.
  # Objects cannot currently be serialized, but we can produce a helpful error.

  def read{{ canonical_type_name }}
    raise InternalError, 'RustBufferStream.read not implemented yet for {{ canonical_type_name }}'
  end

  {% when Type::CallbackInterface with (object_name) -%}
  # The Callback Interface type {{ object_name }}.
  # Objects cannot currently be serialized, but we can produce a helpful error.

  def read{{ canonical_type_name }}
    raise InternalError, 'RustBufferStream.read not implemented yet for {{ canonical_type_name }}'
  end

  {% when Type::Error with (error_name) -%}
  # The Error type {{ error_name }}.
  # Errors cannot currently be serialized, but we can produce a helpful error.

  def read{{ canonical_type_name }}
    raise InternalError, 'RustBufferStream.read not implemented yet for {{ canonical_type_name }}'
  end

  {% when Type::Enum with (enum_name) -%}
  {%- let e = ci.get_enum_definition(enum_name).unwrap() -%}
  # The Enum type {{ enum_name }}.

  def read{{ canonical_type_name }}
    variant = unpack_from 4, 'l>'
    {% if e.is_flat() -%}
    {%- for variant in e.variants() %}
    if variant == {{ loop.index }}
      return {{ enum_name|class_name_rb }}::{{ variant.name()|enum_name_rb }}
    end
    {%- endfor %}

    raise InternalError, 'Unexpected variant tag for {{ canonical_type_name }}'
    {%- else -%}
    {%- for variant in e.variants() %}
    if variant == {{ loop.index }}
        {%- if variant.has_fields() %}
        return {{ enum_name|class_name_rb }}::{{ variant.name()|enum_name_rb }}.new(
            {%- for field in variant.fields() %}
            self.read{{ field.type_().canonical_name()|class_name_rb }}(){% if loop.last %}{% else %},{% endif %}
            {%- endfor %}
        )
        {%- else %}
        return {{ enum_name|class_name_rb }}::{{ variant.name()|enum_name_rb }}.new
        {% endif %}
    end
    {%- endfor %}
    raise InternalError, 'Unexpected variant tag for {{ canonical_type_name }}'
    {%- endif %}
  end

  {% when Type::Record with (record_name) -%}
  {%- let rec = ci.get_record_definition(record_name).unwrap() -%}
  # The Record type {{ record_name }}.

  def read{{ canonical_type_name }}
    {{ rec.name()|class_name_rb }}.new(
      {%- for field in rec.fields() %}
      read{{ field.type_().canonical_name()|class_name_rb }}{% if loop.last %}{% else %},{% endif %}
      {%- endfor %}
    )
  end

  {% when Type::Optional with (inner_type) -%}
  # The Optional<T> type for {{ inner_type.canonical_name() }}.

  def read{{ canonical_type_name }}
    flag = unpack_from 1, 'c'

    if flag == 0
      return nil
    elsif flag == 1
      return read{{ inner_type.canonical_name()|class_name_rb }}
    else
      raise InternalError, 'Unexpected flag byte for {{ canonical_type_name }}'
    end
  end

  {% when Type::Sequence with (inner_type) -%}
  # The Sequence<T> type for {{ inner_type.canonical_name() }}.

  def read{{ canonical_type_name }}
    count = unpack_from 4, 'l>'

    raise InternalError, 'Unexpected negative sequence length' if count.negative?

    items = []

    count.times do
      items.append read{{ inner_type.canonical_name()|class_name_rb }}
    end

    items
  end

  {% when Type::Map with (inner_type) -%}
  # The Map<T> type for {{ inner_type.canonical_name() }}.

  def read{{ canonical_type_name }}
    count = unpack_from 4, 'l>'
    raise InternalError, 'Unexpected negative map size' if count.negative?

    items = {}
    count.times do
      key = readString
      items[key] = read{{ inner_type.canonical_name()|class_name_rb }}
    end

    items
  end
  {%- endmatch -%}
  {%- endfor %}

  def unpack_from(size, format)
    raise InternalError, 'read past end of rust buffer' if @offset + size > @rbuf.len

    value = @rbuf.data.get_bytes(@offset, size).unpack format

    @offset += size

    # TODO: verify this
    raise 'more than one element!!!' if value.size > 1

    value[0]
  end
end
