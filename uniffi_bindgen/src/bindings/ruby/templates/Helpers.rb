def self.uniffi_in_range(i, type_name, min, max)
    i = i.to_i
    if not (min <= i and i < max)
        raise RangeError.new "#{type_name} requires #{min} <= value < #{max}"
    end
    i
end
