# Base class for callback interface FfiConverters.
# Stores Ruby callback objects in a handle map, and converts to/from integer handles.

class CallbackInterfaceFfiConverter
  attr_reader :handle_map

  def initialize
    @handle_map = UniffiHandleMap.new
  end

  def lift(handle)
    @handle_map.get handle
  end

  def read(buf)
    handle = buf.readU64
    lift handle
  end

  def check_lower(_cb)
    # Duck typing - any object with right methods will do.
  end

  def lower(cb)
    @handle_map.insert cb
  end

  def write(cb, buf)
    buf.writeU64 lower cb
  end
end

private_constant :CallbackInterfaceFfiConverter
