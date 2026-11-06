# frozen_string_literal: true

require 'minitest/autorun'
require 'gossip_machines'

class GossipMachinesTest < Minitest::Test
  R = GossipMachines::Rumor
  MAX = (1 << 64) - 1
  U32_MAX = (1 << 32) - 1

  def setup
    @rumor = R.new(peers: [0, MAX], attempt_limit: 1)
  end

  def test_native_api_empty_membership_and_explicit_capacities
    assert_nil R.instance_method(:round).source_location
    assert_match(/\A\d+\.\d+\.\d+\z/, GossipMachines::VERSION)
    assert_equal 64, R::MAX_PEERS
    assert_equal 3, R::FANOUT
    empty = R.new(peers: [], attempt_limit: 1)
    assert_predicate empty, :delivered?
    assert_predicate empty, :exhausted?
    assert_equal [nil, nil, nil], empty.round(0)
    refute empty.acknowledge(0)
    assert_nil empty.status(0)
  end

  def test_sparse_capacity_uses_only_real_members_including_zero
    assert_equal [0, MAX], @rumor.peers
    targets = @rumor.round(MAX)
    assert_equal [0, MAX], targets.compact.sort
    assert_nil targets.last
    assert_equal [1, false], @rumor.status(0)
    assert_equal [1, false], @rumor.status(MAX)
    assert_nil @rumor.status(1)
    assert_equal 1, @rumor.attempt_limit
    assert_predicate @rumor, :exhausted?
    refute_predicate @rumor, :delivered?
    assert_equal [nil, nil, nil], @rumor.round(0)
  end

  def test_untried_peers_are_not_starved_and_rounds_never_repeat_peers
    rumor = R.new(peers: [1, 2, 3, 4, 5], attempt_limit: 2)
    first = rumor.round(7).compact
    second = rumor.round(7).compact
    assert_equal 3, first.uniq.size
    assert_equal 3, second.uniq.size
    assert_equal [1, 2, 3, 4, 5], (first + second).uniq.sort
    refute_predicate rumor, :exhausted?
    2.times { rumor.round(8) }
    assert_predicate rumor, :exhausted?
    rumor.peers.each { |peer| assert_equal [2, false], rumor.status(peer) }
    refute_predicate rumor, :delivered?
  end

  def test_acknowledgements_are_separate_from_attempt_exhaustion
    assert @rumor.acknowledge(0)
    assert_equal [0, true], @rumor.status(0)
    assert @rumor.acknowledge(0)
    refute @rumor.acknowledge(7)
    assert_equal [MAX], @rumor.round(0).compact
    assert_predicate @rumor, :exhausted?
    refute_predicate @rumor, :delivered?
    assert @rumor.acknowledge(MAX)
    assert_predicate @rumor, :delivered?
    assert_equal [nil, nil, nil], @rumor.round(0)
  end

  def test_constructor_validation_and_capacity
    assert_raises(ArgumentError) { R.new(peers: [1, 1], attempt_limit: 1) }
    assert_raises(ArgumentError) { R.new(peers: [], attempt_limit: 0) }
    assert_raises(RangeError) { R.new(peers: [], attempt_limit: U32_MAX + 1) }
    assert_raises(RangeError) { R.new(peers: [], attempt_limit: -1) }
    assert_equal U32_MAX, R.new(peers: [], attempt_limit: U32_MAX).attempt_limit
    all = R::MAX_PEERS.times.to_a
    assert_equal all, R.new(peers: all, attempt_limit: 1).peers
    assert_raises(ArgumentError) { R.new(peers: all + [MAX], attempt_limit: 1) }
    assert_raises(ArgumentError) { R.new(peers: []) }
    assert_raises(ArgumentError) { R.new(peers: [], attempt_limit: 1, typo: true) }
  end

  def test_strict_integer_types_and_invalid_calls_do_not_charge_attempts
    coercible = Object.new
    def coercible.to_int = 1
    [1.0, 1.5, Float::NAN, Float::INFINITY, nil, true, '1', coercible].each do |invalid|
      assert_raises(TypeError) { R.new(peers: [invalid], attempt_limit: 1) }
      assert_raises(TypeError) { R.new(peers: [], attempt_limit: invalid) }
      assert_raises(TypeError) { @rumor.round(invalid) }
      assert_raises(TypeError) { @rumor.acknowledge(invalid) }
      assert_raises(TypeError) { @rumor.status(invalid) }
    end
    [-1, MAX + 1].each do |invalid|
      assert_raises(RangeError) { R.new(peers: [invalid], attempt_limit: 1) }
      assert_raises(RangeError) { @rumor.round(invalid) }
      assert_raises(RangeError) { @rumor.acknowledge(invalid) }
      assert_raises(RangeError) { @rumor.status(invalid) }
    end
    assert_equal [0, false], @rumor.status(0)
    assert_equal [0, false], @rumor.status(MAX)
    refute_predicate @rumor, :exhausted?
  end

  def test_peer_arrays_are_strict_detached_and_frozen_input_is_safe
    coercible = Object.new
    def coercible.to_ary = [1]
    [nil, {}, coercible].each do |invalid|
      assert_raises(TypeError) { R.new(peers: invalid, attempt_limit: 1) }
    end
    input = [1, 2].freeze
    rumor = R.new(peers: input, attempt_limit: 1)
    output = rumor.peers
    output.clear
    assert_equal [1, 2], rumor.peers
    assert_equal [1, 2], input
    status = rumor.status(1)
    status.clear
    assert_equal [0, false], rumor.status(1)
  end

  def test_freeze_guards_all_mutators_even_for_unknown_or_exhausted_peers
    @rumor.freeze
    assert_raises(FrozenError) { @rumor.round(0) }
    assert_raises(FrozenError) { @rumor.acknowledge(9) }
    assert_equal [0, false], @rumor.status(0)
    assert_equal [0, MAX], @rumor.peers
    exhausted = R.new(peers: [], attempt_limit: 1).freeze
    assert_raises(FrozenError) { exhausted.round(0) }
  end

  def test_gc_compaction_and_instance_scoping
    other = R.new(peers: [0, MAX], attempt_limit: 1)
    @rumor.acknowledge(0)
    100.times { R.new(peers: [1, 2, 3], attempt_limit: 1) }
    GC.start
    GC.compact if GC.respond_to?(:compact)
    assert_equal [0, true], @rumor.status(0)
    assert_equal [0, false], other.status(0)
    assert_equal [MAX], @rumor.round(MAX).compact
    assert_equal [0, false], other.status(MAX)
  end
end
