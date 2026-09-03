# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uniffi_bindgen_tests'

class TestExternalTypes < Test::Unit::TestCase
  Ext = UniffiBindgenTestsExternalTypesSource

  def test_ext_record
    rec = Ext::ExternalRec.new(a: 42)
    result = UniffiBindgenTests.roundtrip_ext_record(rec)
    assert_equal 42, result.a
  end

  def test_ext_enum
    result = UniffiBindgenTests.roundtrip_ext_enum(Ext::ExternalEnum::TWO)
    assert_equal Ext::ExternalEnum::TWO, result
  end

  def test_ext_interface
    obj = Ext::ExternalInterface.new(123)
    result = UniffiBindgenTests.roundtrip_ext_interface(obj)
    assert_equal 123, result.get_value
  end

  def test_ext_custom_type
    result = UniffiBindgenTests.roundtrip_ext_custom_type(789)
    assert_equal 789, result
  end

  # Identity imported u64 newtypes skip consumer coerce; defining-crate
  # `uniffi_lower_*` must still run `uniffi_in_range` (including `to_int`).
  def test_ext_custom_type_rejects_negative_like_local
    local = assert_raises(RangeError) { UniffiBindgenTests.roundtrip_custom_type1(-1) }
    imported = assert_raises(RangeError) { UniffiBindgenTests.roundtrip_ext_custom_type(-1) }

    assert_equal local.message, imported.message
    assert_equal "u64 requires 0 <= value < #{2**64}", imported.message
  end

  def test_ext_custom_type_rejects_non_integer_like_local
    local = assert_raises(TypeError) { UniffiBindgenTests.roundtrip_custom_type1('nope') }
    imported = assert_raises(TypeError) { UniffiBindgenTests.roundtrip_ext_custom_type('nope') }

    assert_equal local.message, imported.message
    assert_equal 'no implicit conversion of nope into Integer', imported.message
  end

  # Ruby 4 Float#to_int truncates (1.9 -> 1). Both paths must agree; FFI
  # `:uint64` without `uniffi_in_range` would not go through `to_int`.
  def test_ext_custom_type_coerces_float_like_local
    assert_equal 1, UniffiBindgenTests.roundtrip_custom_type1(1.9)
    assert_equal 1, UniffiBindgenTests.roundtrip_ext_custom_type(1.9)
  end

  def test_ext_custom_type_preserves_to_int_coercion
    int_like = Object.new
    def int_like.to_int
      7
    end

    assert_equal 7, UniffiBindgenTests.roundtrip_custom_type1(int_like)
    assert_equal 7, UniffiBindgenTests.roundtrip_ext_custom_type(int_like)
  end
end
