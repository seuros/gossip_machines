# frozen_string_literal: true

require 'minitest/autorun'
require 'gossip_machines'

class GossipMachinesTest < Minitest::Test
  R = GossipMachines::Rumor
  S = GossipMachines::Status
  MAX = (1 << 64) - 1
  U32_MAX = (1 << 32) - 1

  def setup
    @rumor = R.new([0, MAX], 1)
  end

  def test_native_api_empty_membership_and_capacity
    assert_nil R.instance_method(:round).source_location
    assert_match(/\A\d+\.\d+\.\d+\z/, GossipMachines::VERSION)
    assert_equal 64, R::MAX_PEERS
    assert_equal 64, @rumor.capacity
    refute R.const_defined?(:FANOUT)
    assert_raises(TypeError) { R.allocate }
    empty = R.new([], 1)
    assert_predicate empty, :delivered?
    assert_predicate empty, :exhausted?
    assert_empty empty.round(0, 3)
    assert_equal :unknown_peer, empty.acknowledge(0)
    assert_nil empty.status(0)
  end

  def test_status_mirrors_the_core_as_data
    assert_equal %i[attempts acknowledged], S.members
    assert_equal S.new(0, false), @rumor.status(0)
    assert_equal S.new(attempts: 0, acknowledged: false), @rumor.status(MAX)
    assert_predicate @rumor.status(0), :frozen?
    assert_nil @rumor.status(1)
  end

  def test_sparse_capacity_uses_only_real_members_including_zero
    assert_equal [0, MAX], @rumor.peers
    assert_equal [0, MAX], @rumor.round(MAX, 3).sort
    assert_equal S.new(1, false), @rumor.status(0)
    assert_equal S.new(1, false), @rumor.status(MAX)
    assert_equal 1, @rumor.attempt_limit
    assert_predicate @rumor, :exhausted?
    refute_predicate @rumor, :delivered?
    assert_empty @rumor.round(0, 3)
  end

  def test_round_writes_at_most_max_and_charges_only_what_it_returns
    rumor = R.new([1, 2, 3, 4, 5], 2)
    assert_empty rumor.round(7, 0)
    rumor.peers.each { |peer| assert_equal S.new(0, false), rumor.status(peer) }
    first = rumor.round(7, 3)
    second = rumor.round(7, 3)
    assert_equal [3, 3], [first.uniq.size, second.uniq.size]
    assert_equal [1, 2, 3, 4, 5], (first + second).uniq.sort
    assert_equal 6, rumor.peers.sum { |peer| rumor.status(peer).attempts }
    refute_predicate rumor, :exhausted?
    last = rumor.round(8, MAX)
    assert_equal [4, 4], [last.size, last.uniq.size]
    assert_predicate rumor, :exhausted?
    rumor.peers.each { |peer| assert_equal S.new(2, false), rumor.status(peer) }
    assert_empty rumor.round(8, 64)
    rumor.peers.each { |peer| assert_equal S.new(2, false), rumor.status(peer) }
    refute_predicate rumor, :delivered?
  end

  def test_acknowledge_names_first_repeat_and_stranger
    assert_equal :acknowledged, @rumor.acknowledge(0)
    assert_equal S.new(0, true), @rumor.status(0)
    assert_equal :duplicate, @rumor.acknowledge(0)
    assert_equal :unknown_peer, @rumor.acknowledge(7)
    assert_equal S.new(0, true), @rumor.status(0)
    assert_equal [MAX], @rumor.round(0, 3)
    assert_predicate @rumor, :exhausted?
    refute_predicate @rumor, :delivered?
    assert_equal :acknowledged, @rumor.acknowledge(MAX)
    assert_equal :duplicate, @rumor.acknowledge(MAX)
    assert_equal S.new(1, true), @rumor.status(MAX)
    assert_predicate @rumor, :delivered?
    assert_empty @rumor.round(0, 3)
  end

  def test_constructor_validation_and_capacity
    assert_raises(ArgumentError) { R.new([1, 1], 1) }
    assert_raises(RangeError) { R.new([], 0) }
    assert_raises(RangeError) { R.new([], U32_MAX + 1) }
    assert_raises(RangeError) { R.new([], -1) }
    assert_equal U32_MAX, R.new([], U32_MAX).attempt_limit
    all = R::MAX_PEERS.times.to_a
    assert_equal all, R.new(all, 1).peers
    assert_raises(ArgumentError) { R.new(all + [MAX], 1) }
    assert_raises(ArgumentError) { R.new([]) }
    assert_raises(ArgumentError) { R.new([], 1, 2) }
    assert_raises(ArgumentError) { R.new(peers: [], attempt_limit: 1) }
    assert_raises(ArgumentError) { @rumor.round(0) }
  end

  def test_strict_integer_types_and_invalid_calls_do_not_charge_attempts
    coercible = Object.new
    def coercible.to_int = 1
    [1.0, 1.5, Float::NAN, Float::INFINITY, nil, true, '1', coercible].each do |invalid|
      assert_raises(TypeError) { R.new([invalid], 1) }
      assert_raises(TypeError) { R.new([], invalid) }
      assert_raises(TypeError) { @rumor.round(invalid, 3) }
      assert_raises(TypeError) { @rumor.round(0, invalid) }
      assert_raises(TypeError) { @rumor.acknowledge(invalid) }
      assert_raises(TypeError) { @rumor.status(invalid) }
    end
    [-1, MAX + 1].each do |invalid|
      assert_raises(RangeError) { R.new([invalid], 1) }
      assert_raises(RangeError) { @rumor.round(invalid, 3) }
      assert_raises(RangeError) { @rumor.round(0, invalid) }
      assert_raises(RangeError) { @rumor.acknowledge(invalid) }
      assert_raises(RangeError) { @rumor.status(invalid) }
    end
    assert_equal S.new(0, false), @rumor.status(0)
    assert_equal S.new(0, false), @rumor.status(MAX)
    refute_predicate @rumor, :exhausted?
  end

  def test_peer_arrays_are_strict_detached_and_frozen_input_is_safe
    coercible = Object.new
    def coercible.to_ary = [1]
    [nil, {}, coercible].each do |invalid|
      assert_raises(TypeError) { R.new(invalid, 1) }
    end
    input = [1, 2].freeze
    rumor = R.new(input, 1)
    output = rumor.peers
    output.clear
    assert_equal [1, 2], rumor.peers
    assert_equal [1, 2], input
    rumor.round(0, 2).clear
    assert_equal [1, 2], rumor.peers
  end

  def test_freeze_guards_all_mutators_even_for_unknown_or_exhausted_peers
    @rumor.freeze
    assert_raises(FrozenError) { @rumor.round(0, 3) }
    assert_raises(FrozenError) { @rumor.acknowledge(9) }
    assert_equal S.new(0, false), @rumor.status(0)
    assert_equal [0, MAX], @rumor.peers
    exhausted = R.new([], 1).freeze
    assert_raises(FrozenError) { exhausted.round(0, 3) }
  end

  def test_gc_compaction_and_instance_scoping
    other = R.new([0, MAX], 1)
    @rumor.acknowledge(0)
    100.times { R.new([1, 2, 3], 1) }
    GC.start
    GC.compact if GC.respond_to?(:compact)
    assert_equal S.new(0, true), @rumor.status(0)
    assert_equal S.new(0, false), other.status(0)
    assert_equal [MAX], @rumor.round(MAX, 3)
    assert_equal S.new(0, false), other.status(MAX)
  end
end
