# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'imported_types_transitive'

class TestTransitiveExternalTypes < Test::Unit::TestCase
  class UniffiOneTraitImpl < UniffiOneNs::UniffiOneTrait
    def hello
      'Hello from Ruby transitive consumer'
    end
  end

  # SubLibType lives in crate B; its fields are UniffiOne* types from crate C.
  # Nested (de)serialize is a lexical call in B's mixin, not ancestor lookup.
  def test_sublib_mixin_calls_uniffi_one_lexically
    path = $LOADED_FEATURES.find { |f| File.basename(f) == 'imported_types_sublib.rb' }
    assert_not_nil path, 'imported_types_sublib.rb should be loaded'
    src = File.read(path)

    assert_match(/UniffiOneNs::RustBufferStreamMixin/, src)
    assert_match(/UniffiOneNs::RustBufferBuilderMixin/, src)
    assert_match(/write_TypeUniffiOneEnum/, src)
    assert_match(/read_TypeUniffiOneEnum/, src)
  end

  # external_packages auto-fill still maps uniffi-one for module-name lookup;
  # mixin / require membership is iter_external_types, so this crate must not
  # pull uniffi-one in directly.
  def test_consumer_does_not_require_or_include_transitive_crate
    path = $LOADED_FEATURES.find { |f| File.basename(f) == 'imported_types_transitive.rb' }
    assert_not_nil path, 'imported_types_transitive.rb should be loaded'
    src = File.read(path)

    assert_match(/require ['"]imported_types_sublib['"]/, src)
    assert_not_match(/require ['"]uniffi_one_ns['"]/, src)
    assert_not_match(/include ::UniffiOneNs::/, src)
  end

  def test_roundtrip_sub_default
    sub = ImportedTypesSublib::SubLibType.new(
      maybe_enum: nil,
      maybe_trait: nil,
      maybe_interface: nil
    )
    result = ImportedTypesTransitive.roundtrip_sub(sub)

    assert_nil result.maybe_enum
    assert_nil result.maybe_trait
    assert_nil result.maybe_interface
  end

  def test_roundtrip_sub_with_nested_enum
    sub = ImportedTypesSublib::SubLibType.new(
      maybe_enum: UniffiOneNs::UniffiOneEnum::ONE,
      maybe_trait: nil,
      maybe_interface: nil
    )
    result = ImportedTypesTransitive.roundtrip_sub(sub)

    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result.maybe_enum
    assert_nil result.maybe_trait
    assert_nil result.maybe_interface
  end

  def test_roundtrip_sub_with_nested_objects
    trait_impl = UniffiOneTraitImpl.new
    interface = UniffiOneNs::UniffiOneInterface.new
    sub = ImportedTypesSublib::SubLibType.new(
      maybe_enum: UniffiOneNs::UniffiOneEnum::TWO,
      maybe_trait: trait_impl,
      maybe_interface: interface
    )
    result = ImportedTypesTransitive.roundtrip_sub(sub)

    assert_equal UniffiOneNs::UniffiOneEnum::TWO, result.maybe_enum
    assert_equal 'Hello from Ruby transitive consumer', result.maybe_trait.hello
    assert_instance_of UniffiOneNs::UniffiOneInterface, result.maybe_interface
    assert_equal 1, result.maybe_interface.increment
  end

  def test_roundtrip_wrapper_with_nested_enum
    sub = ImportedTypesSublib::SubLibType.new(
      maybe_enum: UniffiOneNs::UniffiOneEnum::ONE,
      maybe_trait: nil,
      maybe_interface: nil
    )
    result = ImportedTypesTransitive.roundtrip_wrapper(
      ImportedTypesTransitive::Wrapper.new(sub: sub)
    )

    assert_equal UniffiOneNs::UniffiOneEnum::ONE, result.sub.maybe_enum
  end
end
