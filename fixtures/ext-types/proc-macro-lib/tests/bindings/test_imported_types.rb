# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uri'
require 'imported_types_lib'
require 'uniffi_one_ns'

class TestImportedTypes < Test::Unit::TestCase
  def test_combined_type
    ct = ImportedTypesLib.get_combined_type(nil)
    assert_equal 'hello', ct.uot.sval
    assert_equal 'a-guid', ct.guid
    assert_equal 'http://example.com/', ct.url.to_s

    ct2 = ImportedTypesLib.get_combined_type(ct)
    assert_equal ct.uot.sval, ct2.uot.sval
    assert_equal ct.guid, ct2.guid
    assert_equal ct.url.to_s, ct2.url.to_s
  end

  def test_uniffi_one_type
    uot = UniffiOneNs::UniffiOneType.new(sval: 'hello')
    result = ImportedTypesLib.get_uniffi_one_type(uot)
    assert_equal 'hello', result.sval
  end

  def test_optional_vec_external_types
    uot = UniffiOneNs::UniffiOneType.new(sval: 'hello')
    assert_equal uot, ImportedTypesLib.get_maybe_uniffi_one_type(uot)
    assert_nil ImportedTypesLib.get_maybe_uniffi_one_type(nil)
    assert_equal [uot], ImportedTypesLib.get_uniffi_one_types([uot])
    assert_equal [uot, nil], ImportedTypesLib.get_maybe_uniffi_one_types([uot, nil])

    e = UniffiOneNs::UniffiOneEnum::ONE
    assert_equal e, ImportedTypesLib.get_maybe_uniffi_one_enum(e)
    assert_nil ImportedTypesLib.get_maybe_uniffi_one_enum(nil)
    assert_equal [e], ImportedTypesLib.get_uniffi_one_enums([e])
    assert_equal [e, nil], ImportedTypesLib.get_maybe_uniffi_one_enums([e, nil])
  end

  def test_url_custom_type
    url = URI.parse('http://example.com/')
    assert_equal url, ImportedTypesLib.get_url(url)
    assert_equal [url], ImportedTypesLib.get_urls([url])
    assert_equal url, ImportedTypesLib.get_maybe_url(url)
    assert_nil ImportedTypesLib.get_maybe_url(nil)
    assert_equal [url, nil], ImportedTypesLib.get_maybe_urls([url, nil])
  end

  def test_uniffi_one_enum
    e = UniffiOneNs::UniffiOneEnum::ONE
    result = ImportedTypesLib.get_uniffi_one_enum(e)
    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result
  end

  def test_procmacro_types
    t = UniffiOneNs::UniffiOneProcMacroType.new(sval: 'hello')
    assert_equal t, ImportedTypesLib.get_uniffi_one_proc_macro_type(t)

    # alternate call path via uniffi-one's own export
    assert_equal t, UniffiOneNs.get_my_proc_macro_type(t)
  end

  def test_objects_type
    ot = ImportedTypesLib.get_objects_type(nil)
    assert_nil ot.maybe_trait
    assert_nil ot.maybe_interface
    assert_nil ImportedTypesLib.get_uniffi_one_trait(nil)
  end

  def test_async_external_types
    assert_equal UniffiOneNs::UniffiOneEnum::ONE, UniffiOneNs.get_uniffi_one_async
    uot = UniffiOneNs::UniffiOneType.new(sval: 'hello')
    assert_equal uot, ImportedTypesLib.get_uniffi_one_type_async(uot)
  end

  def test_procmacro_custom_types
    guid = ImportedTypesLib.get_guid_procmacro(nil)
    assert_equal guid, ImportedTypesLib.get_guid_procmacro(guid)
  end

  def test_customs
    uuid = ImportedTypesLib.get_uuid(nil)
    assert_equal uuid, ImportedTypesLib.get_uuid(uuid)
    assert_equal 'new', ImportedTypesLib.get_uuid_value(uuid)

    handle = ImportedTypesLib.get_newtype_handle(nil)
    assert_equal handle, ImportedTypesLib.get_newtype_handle(handle)
    assert_equal 42, ImportedTypesLib.get_newtype_handle_value(handle)
  end

  def test_misc
    assert_equal 'hello', ImportedTypesLib.get_ouid2
  end
end
