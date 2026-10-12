# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uniffi_bindgen_tests'

class TestBytes < Test::Unit::TestCase
  include UniffiBindgenTests

  def test_roundtrip_bytes
    data = 'test-data'.b

    assert_equal data, UniffiBindgenTests.roundtrip_bytes(data)
  end

  def test_roundtrip_empty_bytes
    assert_equal ''.b, UniffiBindgenTests.roundtrip_bytes(''.b)
  end

  def test_roundtrip_binary_bytes
    data = "\x00\x01\x02\xFF".b

    assert_equal data, UniffiBindgenTests.roundtrip_bytes(data)
  end

  def test_bytes_encoding
    result = UniffiBindgenTests.roundtrip_bytes('hello'.b)

    assert_equal Encoding::BINARY, result.encoding
  end

  def binary_buf(*bytes)
    String.new(bytes.pack('C*'), encoding: Encoding::BINARY)
  end

  # Zero-copy &[u8]. Frozen BINARY strings are fine (read-only).
  def test_zero_copy_bytes
    assert_equal 0, UniffiBindgenTests.sum_bytes(''.b)
    assert_equal 6, UniffiBindgenTests.sum_bytes("\x01\x02\x03".b)
    assert_equal 6, UniffiBindgenTests.sum_bytes(binary_buf(1, 2, 3))
    assert_equal 6, UniffiBindgenTests.sum_bytes("\x01\x02\x03".b.freeze)
    assert_nil UniffiBindgenTests.first_byte(''.b)
    assert_equal 42, UniffiBindgenTests.first_byte("\x2a".b)
    assert_equal 42, UniffiBindgenTests.first_byte(binary_buf(42))
  end

  def test_zero_copy_bytes_type_error
    assert_raise TypeError do UniffiBindgenTests.sum_bytes(nil) end
    assert_raise TypeError do UniffiBindgenTests.sum_bytes(0) end
  end

  # Zero-copy &mut [u8]. Rust writes land in the caller's unfrozen String.
  def test_zero_copy_bytes_mut
    buf = binary_buf(0, 0, 0, 0)
    UniffiBindgenTests.fill_bytes(buf)
    assert_equal binary_buf(0, 1, 2, 3), buf

    inc = binary_buf(1, 2, 3)
    UniffiBindgenTests.increment_bytes(inc)
    assert_equal binary_buf(2, 3, 4), inc

    empty = binary_buf
    UniffiBindgenTests.fill_bytes(empty)
    assert_equal binary_buf, empty
  end

  def test_zero_copy_bytes_mut_type_error
    assert_raise TypeError do UniffiBindgenTests.fill_bytes(nil) end
    assert_raise TypeError do UniffiBindgenTests.fill_bytes(0) end
    # `String#b` copies and is unfrozen; freeze the buffer for the mut rejection.
    assert_raise TypeError do UniffiBindgenTests.fill_bytes('frozen'.b.freeze) end
  end
end
