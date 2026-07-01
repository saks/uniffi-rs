# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uri'
require 'imported_types_lib'
require 'uniffi_one_ns'
require 'imported_types_sublib'

class TestImportedTypes < Test::Unit::TestCase
  def test_invoke_uniffi_one_trait
    impl = Class.new(UniffiOneNs::UniffiOneTrait) do
      def hello
        'Hello from Ruby'
      end
    end.new
    assert_equal 'Hello from Ruby',
                 ImportedTypesLib.invoke_uniffi_one_trait(impl)
  end

  def test_combined_type
    ct = ImportedTypesLib.get_combined_type(nil)
    assert_equal 'hello', ct.uot.sval
    assert_equal 'a-guid', ct.guid
    assert_equal 'ecd', ct.ecd.sval
    assert_equal 'http://example.com/', ct.url.to_s

    ct2 = ImportedTypesLib.get_combined_type(ct)
    assert_equal ct.uot.sval, ct2.uot.sval
    assert_equal ct.guid, ct2.guid
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

  def test_external_crate_types
    iface = ImportedTypesLib.get_external_crate_interface('foo')
    assert_equal 'foo', iface.value
  end

  def test_uniffi_one_enum
    e = UniffiOneNs::UniffiOneEnum::ONE
    result = ImportedTypesLib.get_uniffi_one_enum(e)
    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result
  end

  def test_objects_type
    ot = ImportedTypesLib.get_objects_type(nil)
    assert_nil ot.maybe_trait
    assert_nil ot.maybe_interface
  end

  def test_procmacro_types
    t = UniffiOneNs::UniffiOneProcMacroType.new(sval: 'hello')
    assert_equal t, ImportedTypesLib.get_uniffi_one_proc_macro_type(t)
  end

  def test_external_errors
    assert_raises(UniffiOneNs::UniffiOneError::Oops) { ImportedTypesLib.throw_uniffi_one_error }
    assert_raises(UniffiOneNs::UniffiOneErrorInterface) { ImportedTypesLib.throw_uniffi_one_error_interface }
  end

  def test_async_external_error
    assert_raises(UniffiOneNs::UniffiOneError::Oops) {
      ImportedTypesLib.throw_uniffi_one_error_async
    }
  end

  def test_optional_trait
    assert_nil ImportedTypesLib.get_uniffi_one_trait(nil)
  end

  def test_imported_custom_types
    assert_equal 'guid', ImportedTypesLib.get_imported_guid('guid')
    assert_equal 'ouid', ImportedTypesLib.get_imported_ouid('ouid')
    assert_equal 3, ImportedTypesLib.get_imported_handle_u8(nil)
  end

  def test_nested_imported_custom_types
    assert_equal 'nested', ImportedTypesLib.get_imported_nested_guid(nil)
    assert_equal 'nested', ImportedTypesLib.get_imported_nested_ouid(nil)
    assert_equal 'nested-external', ImportedTypesLib.get_nested_external_guid(nil)
    assert_equal 'nested-external-ouid', ImportedTypesLib.get_nested_external_ouid(nil)
  end

  def test_rename
    t = ImportedTypesLib.get_binding_renamed_type('external_rename_test')
    assert_equal 'external_rename_test', t.value
  end

  def test_trait_impl
    t = ImportedTypesSublib.get_trait_impl
    assert_equal 'sub-lib trait impl says hello', t.hello
  end

  def test_sub_type_with_trait
    t = ImportedTypesSublib.get_trait_impl
    sub = ImportedTypesSublib::SubLibType.new(maybe_enum: nil, maybe_trait: t, maybe_interface: nil)
    result = ImportedTypesSublib.get_sub_type(sub)
    assert_not_nil result.maybe_trait
  end

  def test_objects_type_with_trait
    t = ImportedTypesSublib.get_trait_impl
    sub = ImportedTypesSublib::SubLibType.new(maybe_enum: nil, maybe_trait: t, maybe_interface: nil)
    ot = ImportedTypesLib.get_objects_type(nil)
    assert_nil ot.maybe_trait
    assert_nil ot.maybe_interface
  end
end
