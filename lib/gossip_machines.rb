# frozen_string_literal: true

require_relative 'gossip_machines/version'

module GossipMachines
  # The core's `Status`: one peer's two ledgers. `attempts` is the sends
  # charged to it so far, `acknowledged` whether it confirmed the rumor.
  # Every pair the core's `Status` fields can hold is valid: `attempts` is an
  # Integer in 0..2**32-1 and `acknowledged` is true or false. A wrong type raises
  # TypeError, a count out of range RangeError.
  Status = Data.define(:attempts, :acknowledged) do
    def initialize(attempts:, acknowledged:)
      attempts, acknowledged = GossipMachines.__send__(:__status, attempts, acknowledged)
      super
    end
  end
end

# A missing native extension is an installation error, not a Ruby fallback.
# It validates and builds Status values.
require 'gossip_machines_native/gossip_machines_native'

module GossipMachines
  private_class_method :__status

  # Status is final. `initialize` is where it validates, and a subclass could
  # override it, or `allocate` build an instance that never ran it:
  # subclassing raises TypeError and `allocate` is gone. The binding only
  # builds Status values; it never reads one back.
  Status.singleton_class.undef_method(:allocate)
  Status.define_singleton_method(:inherited) { |_| raise TypeError, "#{Status} cannot be subclassed" }
end
