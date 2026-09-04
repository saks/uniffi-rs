# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

require 'test/unit'
require 'proc_macro'

include ProcMacro

class TestProcMacroDefaults < Test::Unit::TestCase
  def test_record_with_explicit_defaults
    r = RecordWithDefaults.new no_default_string: 'Test'

    assert_equal 'Test', r.no_default_string
    assert r.boolean
    assert_equal 42, r.integer
    assert_in_delta 4.2, r.float_var, 0.001
    assert_equal [], r.vec
    assert_nil r.opt_vec
    assert_equal 42, r.opt_integer
    assert_equal 42, r.custom_integer
    assert !r.boolean_default
    assert_equal '', r.string_default
    assert_nil r.opt_default
  end

  def test_record_with_implicit_defaults
    r = RecordWithImplicitDefaults.new

    assert !r.boolean
    assert_equal 0, r.int8
    assert_equal 0, r.uint8
    assert_equal 0, r.int16
    assert_equal 0, r.uint16
    assert_equal 0, r.int32
    assert_equal 0, r.uint32
    assert_equal 0, r.int64
    assert_equal 0, r.uint64
    assert_equal 0.0, r.afloat
    assert_equal 0.0, r.adouble
    assert_equal [], r.vec
    assert_equal({}, r.map)
    assert_equal ''.b, r.some_bytes
    assert_nil r.opt_int32
    assert_equal 0, r.custom_integer
  end

  def test_function_defaults
    assert_equal 42, ProcMacro.double_with_default
    assert_equal 1, ProcMacro.sum_with_default(1)
    assert_equal 3, ProcMacro.sum_with_default(1, 2)
  end

  def test_object_defaults
    obj = ObjectWithDefaults.new

    assert_equal 42, obj.add_to_num
    assert_equal 30, obj.add_to_implicit_num
    assert_equal 31, obj.add_to_implicit_num(1)
  end

  def test_make_one
    one = ProcMacro.make_one(123)
    assert_equal 123, one.inner
    assert_equal 123, ProcMacro.one_inner_by_ref(one)
    assert_equal 123, one.get_inner_value
  end

  def test_two
    two = ProcMacro::Two.new(a: 'a')
    assert_equal 'a', ProcMacro.take_two(two)
  end

  def test_record_with_bytes
    rwb = ProcMacro::RecordWithBytes.new(some_bytes: [1, 2, 3].pack('C*'))
    assert_equal [1, 2, 3].pack('C*'), ProcMacro.take_record_with_bytes(rwb)
  end

  def test_object_named_ctor
    obj = ProcMacro::Object.named_ctor(1)
    assert_equal ProcMacro::MaybeBool::UNCERTAIN, obj.is_heavy
    obj2 = ProcMacro::Object.named_ctor(0)
    assert_equal ProcMacro::MaybeBool::UNCERTAIN, obj.is_other_heavy(obj2)
  end

  def test_renamed_object
    robj = ProcMacro::Renamed.new
    assert robj.func
    assert ProcMacro.rename_test
  end

  def test_three
    obj = ProcMacro::Object.named_ctor(0)
    three = ProcMacro::Three.new(obj: obj)
  end

  def test_make_zero
    assert_equal 'ZERO', ProcMacro.make_zero.inner
  end

  def test_make_record_with_bytes
    assert_equal [0, 1, 2, 3, 4].pack('C*'), ProcMacro.make_record_with_bytes.some_bytes
  end

  def test_enums
    assert_equal ProcMacro::MaybeBool::TRUE, ProcMacro.enum_identity(ProcMacro::MaybeBool::TRUE)
  end

  def test_hashmap
    hm = ProcMacro.make_hashmap(1, 2)
    assert_equal({ 1 => 2 }, hm)
    d = { 1 => 2 }
    assert_equal d, ProcMacro.return_hashmap(d)
  end

  def test_hash_set
    hs = ProcMacro.make_hash_set('hello')
    assert_equal Set['hello'], hs
    s = Set['a', 'b', 'c']
    assert_equal s, ProcMacro.return_hash_set(s)
  end

  def test_join
    assert_equal 'a:b:c', ProcMacro.join(['a', 'b', 'c'], ':')
  end

  def test_always_fails
    assert_raise ProcMacro::BasicError::OsError do
      ProcMacro.always_fails
    end
  end

  def test_do_stuff
    obj = ProcMacro::Object.named_ctor(0)
    obj.do_stuff(5)
    assert_raise ProcMacro::FlatError::InvalidInput do
      obj.do_stuff(0)
    end
  end

  def test_get_mixed_enum
    assert_equal ProcMacro::MixedEnum::INT.new(1), ProcMacro.get_mixed_enum(nil)
    assert_equal ProcMacro::MixedEnum::NONE.new, ProcMacro.get_mixed_enum(ProcMacro::MixedEnum::NONE.new)
    assert_equal ProcMacro::MixedEnum::STRING.new('hello'), ProcMacro.get_mixed_enum(ProcMacro::MixedEnum::STRING.new('hello'))
  end

  def test_enum_methods
    # Ruby enums are integer constants — no .next() or .value() methods
    assert_equal 0, ProcMacro::MaybeBool::TRUE
  end

  def test_record_with_defaults_roundtrip
    r1 = ProcMacro::RecordWithDefaults.new(no_default_string: 'Test')
    r2 = ProcMacro::RecordWithDefaults.new(no_default_string: 'Test')
    assert_equal r1, r2

    r3 = ProcMacro::RecordWithDefaults.new(no_default_string: '', vec: ['oops'])
    r4 = ProcMacro::RecordWithDefaults.new(no_default_string: '', vec: ['oops'])
    assert_equal r3, r4
  end

  def test_udl_exposed_functions
    assert_equal 0, ProcMacro.get_one(nil).inner
    assert_equal ProcMacro::MaybeBool::UNCERTAIN, ProcMacro.get_bool(nil)
    assert_equal ProcMacro::MaybeBool::UNCERTAIN, ProcMacro.get_object(nil).is_heavy
    assert_nil ProcMacro.get_externals(nil).one
  end

end
