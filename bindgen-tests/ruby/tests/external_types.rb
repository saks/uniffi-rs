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
end
