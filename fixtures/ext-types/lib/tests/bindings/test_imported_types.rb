# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'imported_types_lib'

class TestImportedTypes < Test::Unit::TestCase
  def test_combined_type
    ct = ImportedTypesLib.get_combined_type(nil)
    assert_equal 'hello', ct.uot.sval
    assert_equal 'a-guid', ct.guid
  end

  def test_uniffi_one_type
    uot = UniffiOneNs::UniffiOneType.new(sval: 'hello')
    result = ImportedTypesLib.get_uniffi_one_type(uot)
    assert_equal 'hello', result.sval
  end

  def test_uniffi_one_enum
    e = UniffiOneNs::UniffiOneEnum::ONE
    result = ImportedTypesLib.get_uniffi_one_enum(e)
    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result
  end
end
