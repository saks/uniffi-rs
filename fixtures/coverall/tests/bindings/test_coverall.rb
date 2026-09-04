# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/. */

require 'test/unit'
require 'coverall'

class TestCoverall < Test::Unit::TestCase
  def test_some_dict
    d = Coverall.create_some_dict
    assert_equal(d.text, 'text')
    assert_equal(d.text.encoding, Encoding::UTF_8)
    assert_equal(d.maybe_text, 'maybe_text')
    assert_equal(d.maybe_text.encoding, Encoding::UTF_8)
    assert_equal(d.some_bytes, 'some_bytes')
    assert_equal(d.some_bytes.encoding, Encoding::BINARY)
    assert_equal(d.maybe_some_bytes, 'maybe_some_bytes')
    assert_equal(d.maybe_some_bytes.encoding, Encoding::BINARY)
    assert_true(d.a_bool)
    assert_true(d.a_bool)
    assert_false(d.maybe_a_bool)
    assert_equal(d.unsigned8, 1)
    assert_equal(d.maybe_unsigned8, 2)
    assert_equal(d.unsigned16, 3)
    assert_equal(d.maybe_unsigned16, 4)
    assert_equal(d.unsigned64, 18_446_744_073_709_551_615)
    assert_equal(d.maybe_unsigned64, 0)
    assert_equal(d.signed8, 8)
    assert_equal(d.maybe_signed8, 0)
    assert_equal(d.signed64, 9_223_372_036_854_775_807)
    assert_equal(d.maybe_signed64, 0)

    assert_in_delta(d.float32, 1.2345)
    assert_in_delta(d.maybe_float32, 22.0 / 7.0)

    assert_equal(d.float64, 0.0)
    assert_equal(d.maybe_float64, 1.0)

    assert_equal(d.coveralls.get_name, 'some_dict')

    assert_equal(d.coveralls_list[0].get_name, 'some_dict_1')
    assert_nil(d.coveralls_list[1])
    assert_equal(d.coveralls_list[2].get_name, 'some_dict_2')

    assert_equal(d.coveralls_map['some_dict_3'].get_name, 'some_dict_3')
    assert_nil(d.coveralls_map['none'])
    assert_equal(d.coveralls_map['some_dict_4'].get_name, 'some_dict_4')

    GC.start
    assert_equal 5, Coverall.get_num_alive
    d = nil
    GC.start
    assert_equal 0, Coverall.get_num_alive
  end

  def test_none_dict
    d = Coverall.create_none_dict
    assert_equal(d.text, 'text')
    assert_equal(d.text.encoding, Encoding::UTF_8)
    assert_nil(d.maybe_text)
    assert_equal(d.some_bytes, 'some_bytes')
    assert_equal(d.some_bytes.encoding, Encoding::BINARY)
    assert_nil(d.maybe_some_bytes)
    assert_true(d.a_bool)
    assert_nil(d.maybe_a_bool)
    assert_equal(d.unsigned8, 1)
    assert_nil(d.maybe_unsigned8)
    assert_equal(d.unsigned16, 3)
    assert_nil(d.maybe_unsigned16)
    assert_equal(d.unsigned64, 18_446_744_073_709_551_615)
    assert_nil(d.maybe_unsigned64)
    assert_equal(d.signed8, 8)
    assert_nil(d.maybe_signed8)
    assert_equal(d.signed64, 9_223_372_036_854_775_807)
    assert_nil(d.maybe_signed64)

    assert_in_delta(d.float32, 1.2345)
    assert_nil(d.maybe_float32)
    assert_equal(d.float64, 0.0)
    assert_nil(d.maybe_float64)

    GC.start
    assert_equal 0, Coverall.get_num_alive
    d = nil
    GC.start
    assert_equal 0, Coverall.get_num_alive
  end

  def test_constructors
    GC.start
    assert_equal(Coverall.get_num_alive, 0)
    # must work.
    coveralls = Coverall::Coveralls.new 'c1'
    assert_equal(Coverall.get_num_alive, 1)
    # make sure it really is our Coveralls object.
    assert_equal(coveralls.get_name, 'c1')
    # must also work.
    coveralls2 = Coverall::Coveralls.fallible_new('c2', false)
    assert_equal(Coverall.get_num_alive, 2)
    # make sure it really is our Coveralls object.
    assert_equal(coveralls2.get_name, 'c2')

    assert_raise Coverall::CoverallError::TooManyHoles do
      Coverall::Coveralls.fallible_new('', true)
    end

    assert_raise Coverall::InternalError do
      Coverall::Coveralls.panicking_new('expected panic: woe is me')
    end

    assert_raise_message(/expected panic: woe is me/) do
      Coverall::Coveralls.panicking_new('expected panic: woe is me')
    end

    begin
      objects = 10.times.map { Coverall::Coveralls.new 'c1' }
      assert_equal 12, Coverall.get_num_alive
      objects = nil
      GC.start
    end

    assert_equal 2, Coverall.get_num_alive
  end

  def test_simple_errors
    coveralls = Coverall::Coveralls.new 'test_simple_errors'
    assert_equal coveralls.get_name, 'test_simple_errors'

    err = assert_raise Coverall::CoverallError::TooManyHoles do
      coveralls.maybe_throw true
    end
    assert_equal err.message, 'The coverall has too many holes'

    assert_raise Coverall::CoverallError::TooManyHoles do
      coveralls.maybe_throw_into true
    end

    err = assert_raise Coverall::InternalError do
      coveralls.panic 'expected panic: oh no'
    end
    assert_equal err.message, 'expected panic: oh no'

    assert_raise_message(/expected panic: oh no/) do
      coveralls.panic 'expected panic: oh no'
    end
  end

  def test_complex_errors
    coveralls = Coverall::Coveralls.new 'test_complex_errors'
    assert_equal coveralls.maybe_throw_complex(0), true

    begin
      coveralls.maybe_throw_complex(1)
    rescue Coverall::ComplexError::OsError => e
      assert_equal e.code, 10
      assert_equal e.extended_code, 20
      assert_equal e.to_s, 'Coverall::ComplexError::OsError(code=10, extended_code=20)'
    else
      raise 'should have thrown'
    end

    begin
      coveralls.maybe_throw_complex(2)
    rescue Coverall::ComplexError::PermissionDenied => e
      assert_equal e.reason, 'Forbidden'
      assert_equal e.to_s, 'Coverall::ComplexError::PermissionDenied(reason="Forbidden")'
    else
      raise 'should have thrown'
    end

    begin
      coveralls.maybe_throw_complex(3)
    rescue Coverall::ComplexError::UnknownError => e
      assert_equal e.to_s, 'Coverall::ComplexError::UnknownError()'
    else
      raise 'should have thrown'
    end

    assert_raise Coverall::InternalError do
      coveralls.maybe_throw_complex(4)
    end
  end

  def test_self_by_arc
    coveralls = Coverall::Coveralls.new 'test_self_by_arc'

    # One reference is held by the handlemap, and one by the `Arc<Self>` method receiver.
    assert_equal coveralls.strong_count, 2
  end

  def test_arcs
    GC.start
    coveralls = Coverall::Coveralls.new 'test_arcs'
    assert_equal 1, Coverall.get_num_alive

    assert_equal 2, coveralls.strong_count
    assert_equal nil, coveralls.get_other

    coveralls.take_other coveralls
    # should now be a new strong ref.
    assert_equal 3, coveralls.strong_count
    # but the same number of instances.
    assert_equal 1, Coverall.get_num_alive
    # and check it's the correct object.
    assert_equal 'test_arcs', coveralls.get_other.get_name

    # Using `assert_raise` here would keep a reference to `coveralls` alive
    # by capturing it in a closure, which would interfere with the tests.
    begin
      coveralls.take_other_fallible
    rescue Coverall::CoverallError::TooManyHoles
      # OK
    else
      raise 'should have thrown'
    end

    begin
      coveralls.take_other_panic 'expected panic: with an arc!'
    rescue Coverall::InternalError => e
      assert_match(/expected panic: with an arc!/, e.message)
    else
      raise 'should have thrown'
    end

    coveralls.take_other nil
    GC.start
    assert_equal 2, coveralls.strong_count

    # Reference cleanup includes the cached most recent exception.
    coveralls = nil
    GC.start
    assert_equal 0, Coverall.get_num_alive
  end

  def test_return_objects
    GC.start
    coveralls = Coverall::Coveralls.new 'test_return_objects'
    assert_equal Coverall.get_num_alive, 1
    assert_equal coveralls.strong_count, 2
    c2 = coveralls.clone_me
    assert_equal c2.get_name, coveralls.get_name
    assert_equal Coverall.get_num_alive, 2
    assert_equal c2.strong_count, 2

    coveralls.take_other(c2)
    # same number alive but `c2` has an additional ref count.
    assert_equal Coverall.get_num_alive, 2
    assert_equal coveralls.strong_count, 2
    assert_equal c2.strong_count, 3

    # We can drop Ruby's reference to `c2`, but the Rust struct will not
    # be dropped as coveralls hold an `Arc<>` to it.
    c2 = nil
    GC.start
    assert_equal Coverall.get_num_alive, 2

    # Dropping `coveralls` will kill both.
    coveralls = nil
    GC.start
    assert_equal Coverall.get_num_alive, 0
  end

  def test_bad_objects
    coveralls = Coverall::Coveralls.new 'test_bad_objects'
    patch = Coverall::Patch.new Coverall::Color::RED
    # `coveralls.take_other` wants `Coveralls` not `Patch`
    assert_raise_message(/Expected a Coveralls instance, got.*Patch/) do
      coveralls.take_other patch
    end
  end

  def test_flat_errors
    err = assert_raise Coverall::CoverallFlatError::TooManyVariants do
      Coverall.throw_flat_error
    end
    assert_equal err.message, 'Too many variants: 99'
  end

  def test_flat_macro_errors
    err = assert_raise Coverall::CoverallFlatMacroError::TooManyVariants do
      Coverall.throw_flat_macro_error
    end
    assert_equal err.message, 'Too many variants: 88'
  end

  def test_rich_error_no_variant_data
    assert_raise Coverall::CoverallRichErrorNoVariantData::TooManyPlainVariants do
      Coverall.throw_rich_error_no_variant_data
    end
  end

  def test_macro_errors
    err = assert_raise Coverall::CoverallMacroError::TooManyMacros do
      Coverall.throw_macro_error
    end
    assert_equal err.message, 'The coverall has too many macros'
  end

  def test_complex_macro_errors
    err = assert_raise Coverall::ComplexMacroError::OsError do
      Coverall.throw_complex_macro_error
    end
    assert_equal err.code, 1
    assert_equal err.extended_code, 2
  end

  def test_error_values
    begin
      Coverall.throw_root_error
    rescue Coverall::RootError::Complex => e
      assert_equal e.error.code, 1
    else
      raise 'should have thrown'
    end

    e = Coverall.get_root_error
    assert_equal e.error, Coverall::OtherError::UNEXPECTED

    result = Coverall.get_complex_error(nil)
    assert_true result.is_a?(Coverall::ComplexError::PermissionDenied)

    d = Coverall.get_error_dict(nil)
    assert_nil d.complex_error
  end

  def test_enums
    e = Coverall.get_simple_flat_macro_enum(0)
    assert_true e.is_a?(Coverall::SimpleFlatMacroEnum::FIRST)

    sd = Coverall.get_maybe_simple_dict(0)
    assert_true sd.yeah?
    assert_equal sd.d.text, ''

    sd2 = Coverall.get_maybe_simple_dict(1)
    assert_true sd2.nah?

    mo = Coverall.get_maybe_object(0)
    assert_true mo.obj?
    assert_equal mo.p.get_color, Coverall::Color::RED

    mo2 = Coverall.get_maybe_object(1)
    assert_true mo2.nah?
  end

  def test_dict_with_defaults
    d = Coverall::DictWithDefaults.new
    assert_equal 'default-value', d.name
    assert_nil d.category
    assert_equal 31, d.integer
    assert_equal [], d.item_list
    assert_equal({}, d.item_map)

    d2 = Coverall::DictWithDefaults.new(name: 'this', category: 'that', integer: 42)
    assert_equal 'this', d2.name
    assert_equal 'that', d2.category
    assert_equal 42, d2.integer
  end

  def test_dict_with_non_string_keys
    coveralls = Coverall::Coveralls.new 'test_dict'

    dict1 = coveralls.get_dict('answer', 42)
    assert_equal 42, dict1['answer']

    dict2 = coveralls.get_dict2('answer', 42)
    assert_equal 42, dict2['answer']

    dict3 = coveralls.get_dict3(31, 42)
    assert_equal 42, dict3[31]
  end

  def test_return_only_dict
    err_instance = Coverall::CoverallFlatError::TooManyVariants.new('99')
    assert_raise Coverall::InternalError do
      Coverall.try_input_return_only_dict(Coverall::ReturnOnlyDict.new(e: err_instance))
    end
  end

  def test_throwing_constructors
    assert_raise Coverall::CoverallError::TooManyHoles do
      Coverall::FalliblePatch.new
    end
    assert_raise Coverall::CoverallError::TooManyHoles do
      Coverall::FalliblePatch.secondary
    end
  end

  def test_patches_and_repairs
    coveralls = Coverall::Coveralls.new 'test_patches_and_repairs'
    coveralls.add_patch Coverall::Patch.new(Coverall::Color::RED)
    coveralls.add_repair Coverall::Repair.new(
      _when: Time.now,
      patch: Coverall::Patch.new(Coverall::Color::BLUE)
    )
    assert_equal 2, coveralls.get_repairs.length
  end

  def test_return_objects_with_repairs
    coveralls = Coverall::Coveralls.new 'test_return_objects'
    assert_equal 1, Coverall.get_num_alive
    assert_equal 2, coveralls.strong_count

    c2 = coveralls.clone_me
    assert_equal c2.get_name, coveralls.get_name
    assert_equal 2, Coverall.get_num_alive
    assert_equal 2, c2.strong_count

    coveralls.take_other c2
    assert_equal 2, Coverall.get_num_alive
    assert_equal 2, coveralls.strong_count
    assert_equal 3, c2.strong_count

    c2 = nil
    GC.start
    assert_equal 2, Coverall.get_num_alive

    coveralls = nil
    GC.start
    assert_equal 0, Coverall.get_num_alive
  end

  def test_bytes
    coveralls = Coverall::Coveralls.new 'test_bytes'
    assert_equal coveralls.reverse('123'), '321'
    assert_equal coveralls.reverse('123').encoding, Encoding::BINARY
  end

  def test_html_error
    assert_raise Coverall::HtmlError::InvalidHtml do
      Coverall.validate_html('test')
    end
  end

  def test_foreign_getters
    Coverall.test_getters(RubyGetters.new)
  end

  def test_foreign_getters_detailed
    g = RubyGetters.new
    assert_equal false, g.get_bool(true, true)
    assert_equal true, g.get_bool(true, false)
    assert_equal 'hello', g.get_string('hello', false)
    assert_equal 'HELLO', g.get_string('hello', true)
    assert_equal 'HELLO', g.get_option('hello', true)
    assert_equal 'hello', g.get_option('hello', false)
    assert_nil g.get_option('', true)
    assert_equal [1, 2, 3], g.get_list([1, 2, 3], true)
    assert_equal [], g.get_list([1, 2, 3], false)
    assert_nil g.get_nothing('hello')
  end

  def test_foreign_getters_errors
    g = RubyGetters.new
    assert_raise Coverall::CoverallError::TooManyHoles do
      g.get_string('too-many-holes', true)
    end
    begin
      g.get_option('os-error', true)
    rescue Coverall::ComplexError::OsError => e
      assert_equal 100, e.code
      assert_equal 200, e.extended_code
    else
      raise 'should have thrown'
    end
    assert_raise Coverall::ComplexError::UnknownError do
      g.get_option('unknown-error', true)
    end
  end

  def test_foreign_getters_round_trip_rust
    Coverall.test_round_trip_through_rust(Coverall.make_rust_getters)
  end

  def test_foreign_getters_round_trip_foreign
    Coverall.test_round_trip_through_foreign(RubyGetters.new)
  end

  def test_rust_only_traits
    traits = Coverall.get_string_util_traits
    assert_equal 'cowboy', traits[0].concat('cow', 'boy')
    assert_equal 'cowboy', traits[1].concat('cow', 'boy')
  end

  def test_pass_object_to_function_that_input_trait
    obj = Coverall::StringUtilObject.new('--')
    assert_raise TypeError do
      Coverall.concat_with_string_util(obj, 'cow', 'boy')
    end
  end

  def test_path
    traits = Coverall.get_traits
    assert_equal 'node-1', traits[0].name
    assert_equal 'node-2', traits[1].name
    traits[0].set_parent(traits[1])
    assert_equal ['node-2'], Coverall.ancestor_names(traits[0])
    assert_equal [], Coverall.ancestor_names(traits[1])
    traits[1].set_parent(nil)
    traits[0].set_parent(nil)
  end

  def test_struct_traits
    node = Coverall::Node.new('test-node')
    assert_true node.get_parent.is_a?(Coverall::NodeTrait)
  end
end

# -- Trait implementations --

class RubyGetters < Coverall::Getters
  def get_bool(v, arg2)
    v ^ arg2
  end

  def get_string(v, arg2)
    if v == 'too-many-holes'
      raise Coverall::CoverallError::TooManyHoles
    elsif v == 'unexpected-error'
      raise 'unexpected error'
    elsif arg2
      v.upcase
    else
      v
    end
  end

  def get_option(v, arg2)
    if v == 'os-error'
      raise Coverall::ComplexError::OsError.new(code: 100, extended_code: 200)
    elsif v == 'unknown-error'
      raise Coverall::ComplexError::UnknownError
    elsif arg2
      v.nil? || v.empty? ? nil : v.upcase
    else
      v
    end
  end

  def get_list(v, arg2)
    arg2 ? v : []
  end

  def get_nothing(_v)
    nil
  end

  def round_trip_object(coveralls)
    coveralls
  end
end

class RubyNode < Coverall::NodeTrait
  def initialize
    @parent = nil
  end

  def name
    'node-rb'
  end

  def set_parent(parent)
    @parent = parent
  end

  def get_parent
    @parent
  end

  def strong_count
    0
  end
end
