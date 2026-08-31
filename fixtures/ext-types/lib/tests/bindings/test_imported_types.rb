# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uri'
require 'imported_types_lib'

class TestImportedTypes < Test::Unit::TestCase
  class UniffiOneTraitImpl < UniffiOneNs::UniffiOneTrait
    def hello
      'Hello from Ruby'
    end
  end

  def test_invoke_uniffi_one_trait
    impl = UniffiOneTraitImpl.new

    assert_equal 'Hello from Ruby', ImportedTypesLib.invoke_uniffi_one_trait(impl)
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
    url = URI.parse 'http://example.com/'

    assert_equal url, ImportedTypesLib.get_url(url)
    assert_equal [url], ImportedTypesLib.get_urls([url])
    assert_equal url, ImportedTypesLib.get_maybe_url(url)
    assert_nil ImportedTypesLib.get_maybe_url(nil)
    assert_equal [url, nil], ImportedTypesLib.get_maybe_urls([url, nil])
  end

  # Imported Url must be a URI on both the top-level FFI path (get_url) and
  # nested mixin paths (get_urls, CombinedType), without a consumer-side
  # custom_types.Url copy.
  def test_imported_url_is_uri
    url = URI.parse 'http://example.com/'

    assert_kind_of URI, ImportedTypesLib.get_url(url)
    assert_kind_of URI, ImportedTypesLib.get_urls([url]).fetch(0)

    ct = ImportedTypesLib.get_combined_type(nil)
    assert_kind_of URI, ct.url
  end

  # LocalUrl wraps imported Url. The defining crate's URI conversion must
  # apply on both the top-level FFI path and the nested mixin (record field)
  # path — otherwise the same type lifts to String vs URI.
  def test_local_url_wrapping_imported_url
    url = URI.parse 'http://example.com/'

    top = ImportedTypesLib.get_local_url(nil)
    assert_kind_of URI, top
    assert_equal url, ImportedTypesLib.get_local_url(url)

    holder = ImportedTypesLib.get_local_url_holder(nil)
    assert_kind_of URI, holder.url
    holder2 = ImportedTypesLib.get_local_url_holder(holder)
    assert_equal holder.url, holder2.url
  end

  def test_external_crate_types
    iface = ImportedTypesLib.get_external_crate_interface 'foo'

    assert_equal 'foo', iface.value
  end

  def test_uniffi_one_enum
    e = UniffiOneNs::UniffiOneEnum::ONE
    result = ImportedTypesLib.get_uniffi_one_enum(e)

    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result
  end

  # Mixin readers live in the defining crate, so a corrupt buffer for an
  # external type raises that crate's InternalError — matching Python/Kotlin
  # converters. ImportedTypesLib::InternalError is a different class.
  def test_corrupt_external_enum_raises_defining_crate_internal_error
    buf = ImportedTypesLib::RustBuffer.alloc(4)
    buf.len = 4
    buf.data.put_bytes(0, [99].pack('l>'))
    err = assert_raise(UniffiOneNs::InternalError) do
      buf.consume_into_TypeUniffiOneEnum
    end
    assert_match(/Unexpected variant tag/, err.message)
    assert_not_same ImportedTypesLib::InternalError, UniffiOneNs::InternalError
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
    assert_raises(UniffiOneNs::UniffiOneErrorInterface) do
      ImportedTypesLib.throw_uniffi_one_error_interface
    end
  end

  def test_async_external_error
    assert_raises(UniffiOneNs::UniffiOneError::Oops) {
      ImportedTypesLib.throw_uniffi_one_error_async
    }
  end

  def test_async_external_types
    assert_equal 'nested-external-ouid', ImportedTypesLib.get_nested_external_ouid_async(nil)
    assert_equal 'local-external-guid', ImportedTypesLib.get_local_external_guid_async
  end

  def test_optional_trait
    assert_nil ImportedTypesLib.get_uniffi_one_trait(nil)
  end

  def test_imported_custom_types
    assert_equal 'guid', ImportedTypesLib.get_imported_guid('guid')
    assert_equal 'ouid', ImportedTypesLib.get_imported_ouid('ouid')
    assert_equal 3, ImportedTypesLib.get_imported_handle_u8(nil)
  end

  def test_direct_custom_types
    assert_equal 'guid', ExtTypesCustom.get_guid('guid')
    assert_equal 'ouid', ExtTypesCustom.get_ouid('ouid')
    assert_equal 'uuid', ExtTypesCustom.get_nested_guid('uuid')
  end

  def test_nested_imported_custom_types
    assert_equal 'nested', ImportedTypesLib.get_imported_nested_guid(nil)
    assert_equal 'nested', ImportedTypesLib.get_imported_nested_ouid(nil)
    assert_equal 'nested-external', ImportedTypesLib.get_nested_external_guid(nil)
    assert_equal 'nested-external-ouid', ImportedTypesLib.get_nested_external_ouid(nil)
  end

  # NestedObject wraps InnerObject. Lowering must qualify
  # ExtTypesCustom::InnerObject.uniffi_lower — an unqualified InnerObject
  # NameErrors inside ImportedTypesLib.
  def test_imported_nested_object
    obj = ExtTypesCustom::InnerObject.new
    result = ImportedTypesLib.get_imported_nested_object(obj)
    assert_instance_of ExtTypesCustom::InnerObject, result
    assert_raise(TypeError) { ImportedTypesLib.get_imported_nested_object('nope') }
  end

  def test_imported_nested_record
    rec = ExtTypesCustom::InnerRecord.new(i: 1)
    result = ImportedTypesLib.get_imported_nested_record(rec)
    assert_equal 1, result.i
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

    assert_instance_of UniffiOneNs::UniffiOneTrait, result.maybe_trait
  end

  def test_objects_type_with_trait
    t = ImportedTypesSublib.get_trait_impl
    sub = ImportedTypesSublib::SubLibType.new(maybe_enum: nil, maybe_trait: t, maybe_interface: nil)
    ot = ImportedTypesLib::ObjectsType.new(maybe_trait: t, maybe_interface: nil, sub: sub)
    result = ImportedTypesLib.get_objects_type(ot)

    assert_equal 'sub-lib trait impl says hello', result.maybe_trait.hello
    assert_nil result.maybe_interface
  end

  def test_takes_external_error
    err = ImportedTypesSublib::NotToThrowError::Variant.new 42

    assert_nothing_raised { ImportedTypesLib.takes_external_error(err) }
  end

  # Generated cross-module references must be rooted at Object (`::UniffiOneNs`).
  # A nested constant of the same name inside the consumer module would
  # otherwise hijack relative lookup in lift/lower/check.
  def test_external_module_lookup_ignores_nested_shadow
    ImportedTypesLib.const_set(:UniffiOneNs, Module.new)

    uot = ::UniffiOneNs::UniffiOneType.new(sval: 'hello')
    assert_equal 'hello', ImportedTypesLib.get_uniffi_one_type(uot).sval

    e = ::UniffiOneNs::UniffiOneEnum::ONE
    assert_equal e, ImportedTypesLib.get_uniffi_one_enum(e)

    impl = UniffiOneTraitImpl.new
    assert_equal 'Hello from Ruby', ImportedTypesLib.invoke_uniffi_one_trait(impl)
  ensure
    if ImportedTypesLib.const_defined?(:UniffiOneNs, false)
      ImportedTypesLib.send(:remove_const, :UniffiOneNs)
    end
  end
end
