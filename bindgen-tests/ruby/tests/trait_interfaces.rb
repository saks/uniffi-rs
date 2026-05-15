# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uniffi_bindgen_tests'

class TraitImpl
  attr_accessor :value

  @@ref_count = 0

  def self.ref_count
    @@ref_count
  end

  def self.finalizer
    proc { @@ref_count -= 1 }
  end

  def initialize(value)
    @value = value
    @@ref_count += 1
    ObjectSpace.define_finalizer(self, self.class.finalizer)
  end

  def noop; end

  def get_value
    value
  end

  def set_value(value)
    self.value = value
  end

  def throw_if_equal(numbers)
    raise UniffiBindgenTests::TestError::Failure1 if numbers.a == numbers.b

    numbers
  end
end

class TestTraitInterfaces < Test::Unit::TestCase
  include UniffiBindgenTests

  def check_rust_impl(trait_impl)
    trait_impl.noop
    assert_equal 42, trait_impl.get_value
    trait_impl.set_value(43)
    assert_equal 43, trait_impl.get_value
    assert_raises(TestError::Failure1) do
      trait_impl.throw_if_equal(CallbackInterfaceNumbers.new(a: 10, b: 10))
    end
    numbers = CallbackInterfaceNumbers.new(a: 10, b: 11)
    assert_equal numbers, trait_impl.throw_if_equal(numbers)
  end

  def check_rb_impl(trait_impl)
    UniffiBindgenTests.invoke_test_trait_interface_noop(trait_impl)
    assert_equal 42, UniffiBindgenTests.invoke_test_trait_interface_get_value(trait_impl)
    UniffiBindgenTests.invoke_test_trait_interface_set_value(trait_impl, 43)
    assert_equal 43, UniffiBindgenTests.invoke_test_trait_interface_get_value(trait_impl)
    assert_raises(TestError::Failure1) do
      UniffiBindgenTests.invoke_test_trait_interface_throw_if_equal(
        trait_impl,
        CallbackInterfaceNumbers.new(a: 10, b: 10)
      )
    end
    numbers = CallbackInterfaceNumbers.new(a: 10, b: 11)
    assert_equal numbers, UniffiBindgenTests.invoke_test_trait_interface_throw_if_equal(trait_impl, numbers)
  end

  def test_rust_impl
    check_rust_impl(UniffiBindgenTests.create_test_trait_interface(42))
  end

  def test_rust_impl_roundtripped
    check_rust_impl(
      UniffiBindgenTests.roundtrip_test_trait_interface(
        UniffiBindgenTests.create_test_trait_interface(42)
      )
    )
  end

  def test_rust_impl_roundtripped_list
    check_rust_impl(
      UniffiBindgenTests.roundtrip_test_trait_interface_list(
        [UniffiBindgenTests.create_test_trait_interface(42)]
      )[0]
    )
  end

  def test_rb_impl
    check_rb_impl(TraitImpl.new(42))
  end

  def test_rb_impl_roundtripped
    impl = UniffiBindgenTests.roundtrip_test_trait_interface(TraitImpl.new(42))
    check_rb_impl(impl)
    impl = nil
    GC.start
    assert_equal 0, TraitImpl.ref_count
  end

end
