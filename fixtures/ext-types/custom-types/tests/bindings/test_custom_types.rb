# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/. */

require 'test/unit'
require 'ext_types_custom'

class TestCustomTypes < Test::Unit::TestCase

  # -- Basic lift/lower --
  
  def test_get_guid
    assert_equal 'NewGuid', ExtTypesCustom.get_guid(nil)
    assert_equal 'SomeGuid', ExtTypesCustom.get_guid('SomeGuid')
    assert_equal 'Ouid', ExtTypesCustom.get_ouid(nil)
  end
  
  # -- Custom types inside a Record and a Sequence --

  def test_guid_helper
    helper = ExtTypesCustom.get_guid_helper(nil)
    assert_equal 'first-guid', helper.guid
    assert_equal ['second-guid', 'third-guid'], helper.guids
    assert_nil helper.maybe_guid

    # Round-trip: pass the same record back in
    helper2 = ExtTypesCustom.get_guid_helper(helper)
    assert_equal helper.guid, helper2.guid
    assert_equal helper.guids, helper2.guids
    assert_equal helper.maybe_guid, helper2.maybe_guid
  end

  # -- Error when lifting fails (no Result return type -> InternalError panic) --

  def test_get_guid_errors
    assert_raise(ExtTypesCustom::InternalError) { ExtTypesCustom.get_guid('') }
    assert_raise(ExtTypesCustom::InternalError) { ExtTypesCustom.get_guid('unexpected') }
    assert_raise(ExtTypesCustom::InternalError) { ExtTypesCustom.get_guid('panic') }
  end

  # -- Error when lifting fails and function returns Result --

  def test_try_get_guid_errors
    # Empty string -> declared Error (GuidError::TooShort)
    assert_raise(ExtTypesCustom::GuidError::TooShort) { ExtTypesCustom.try_get_guid('') }

    # "unexpected" triggers an InternalError (not declared in the throws clause)
    assert_raise(ExtTypesCustom::InternalError) { ExtTypesCustom.try_get_guid('unexpected') }

    # "panic" triggers a Rust panic -> InternalError
    assert_raise(ExtTypesCustom::InternalError) { ExtTypesCustom.try_get_guid('panic') }
  end

  # -- Nested custom types (ANestedGuid wraps Guid wraps String) --
  
  def test_nested_custom_type
    result = ExtTypesCustom.get_nested_guid nil
    assert_equal 'ANestedGuid', result

    result2 = ExtTypesCustom.get_nested_guid result
    assert_equal result, result2
  end

  # -- HandleU8 (custom_newtype over u8 = Integer --

  def test_handle_u8
    assert_equal 2, ExtTypesCustom.get_handle_u8(nil)
    assert_equal 42, ExtTypesCustom.get_handle_u8(42)
  end

  # -- NestedObject / NestedRecord (identity custom types over Object / Record) --

  def test_nested_object
    obj = ExtTypesCustom::InnerObject.new
    result = ExtTypesCustom.get_nested_object(obj)
    assert_instance_of ExtTypesCustom::InnerObject, result
  end

  def test_nested_record
    rec = ExtTypesCustom::InnerRecord.new(i: 1)
    result = ExtTypesCustom.get_nested_record(rec)
    assert_equal 1, result.i
  end

  # -- GuidCallback (custom type passed through callback interface) --

  class GuidCallbackImpl < ExtTypesCustom::GuidCallback
    attr_reader :saw_guid

    def run(guid)
      @saw_guid = guid
      guid
    end
  end

  def test_guid_callback
    impl = GuidCallbackImpl.new

    result = ExtTypesCustom.run_callback(impl)

    assert_equal 'callback-test-payload', result
    assert_equal 'callback-test-payload', impl.saw_guid
  end
end
