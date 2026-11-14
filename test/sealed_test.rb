# frozen_string_literal: true

require 'minitest/autorun'
require 'gossip_machines'

# Status is sealed. Sealing stops accidents, not reflection; the binding only
# builds Status values (through `Status.new`, resolved at init) and never
# reads one back, so a forged Status has no way into the core.
class SealedTest < Minitest::Test
  S = GossipMachines::Status

  # A named subclass raises, but Ruby binds the constant before `inherited`
  # runs, so `ForgedStatus` leaks.
  begin
    class ForgedStatus < GossipMachines::Status; end
  rescue TypeError => e
    NAMED_SUBCLASS_ERROR = e
  end

  def test_status_is_sealed
    assert_raises(NoMethodError) { S.allocate }
    assert_raises(TypeError) { Class.new(S) }
    assert_kind_of TypeError, NAMED_SUBCLASS_ERROR
    assert_equal S, ForgedStatus.superclass, 'the constant leaks'
  end

  def test_status_answers_are_built_through_status_new
    rumor = GossipMachines::Rumor.new([7], 2)
    answer = rumor.status(7)
    assert_instance_of S, answer
    assert_equal S.new(0, false), answer
  end
end
