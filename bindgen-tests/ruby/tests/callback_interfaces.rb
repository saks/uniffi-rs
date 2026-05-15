# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'uniffi_bindgen_tests'

class CallbackImpl
  attr_accessor :value

  @@callback_ref_count = 0

  def self.callback_ref_count
    @@callback_ref_count
  end

  def self.finalizer
    proc { @@callback_ref_count -= 1 }
  end

  def initialize(value)
    @value = value
    @@callback_ref_count += 1
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
    if numbers.a == 6 && numbers.b == 7
      raise 'unexpected failure'
    elsif numbers.a == numbers.b
      raise UniffiBindgenTests::TestError::Failure1
    end

    numbers
  end

  def echo(s)
    s
  end
end

class TestCallbackInterfaces < Test::Unit::TestCase
  include UniffiBindgenTests

  def test_noop
    cbi = CallbackImpl.new 0

    UniffiBindgenTests.invoke_test_callback_interface_noop(cbi)
  end

  def test_get_value
    cbi = CallbackImpl.new 42

    assert_equal 42, UniffiBindgenTests.invoke_test_callback_interface_get_value(cbi)
  end

  def test_set_value
    cbi = CallbackImpl.new 0

    UniffiBindgenTests.invoke_test_callback_interface_set_value(cbi, 43)
    assert_equal 43, UniffiBindgenTests.invoke_test_callback_interface_get_value(cbi)
  end

  def test_echo
    cbi = CallbackImpl.new 0

    assert_equal 'test-string', UniffiBindgenTests.invoke_test_callback_interface_echo(cbi, 'test-string')
    assert_equal '', UniffiBindgenTests.invoke_test_callback_interface_echo(cbi, '')
    assert_equal '*' * 1000, UniffiBindgenTests.invoke_test_callback_interface_echo(cbi, '*' * 1000)
  end

  def test_throw
    cbi = CallbackImpl.new 0

    result = UniffiBindgenTests.invoke_test_callback_interface_throw_if_equal(
      cbi, CallbackInterfaceNumbers.new(a: 1, b: 2)
    )

    assert_equal 1, result.a
    assert_equal 2, result.b
  end

  def test_raises_unexpected_error
    cbi = CallbackImpl.new 0
    numbers = CallbackInterfaceNumbers.new a: 1, b: 1

    assert_raises(TestError::Failure1) do
      UniffiBindgenTests.invoke_test_callback_interface_throw_if_equal cbi, numbers
    end
  end

  def test_raises_unexpected_error_converted
    cbi = CallbackImpl.new 0
    numbers = CallbackInterfaceNumbers.new a: 6, b: 7

    err = assert_raises(TestError::Failure2) do
      UniffiBindgenTests.invoke_test_callback_interface_throw_if_equal cbi, numbers
    end

    assert_kind_of String, err.data
    assert_match(/unexpected failure/, err.data)
  end

  def test_callback_interface_calls_and_errors
    cbi = CallbackImpl.new(42)

    # Only our local Ruby reference should remain alive.
    assert_equal 1, CallbackImpl.callback_ref_count
  end
end
