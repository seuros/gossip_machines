# frozen_string_literal: true

require_relative 'gossip_machines/version'

module GossipMachines
  # The core's `Status`: one peer's two ledgers. `attempts` is the sends
  # charged to it so far, `acknowledged` whether it confirmed the rumor.
  Status = Data.define(:attempts, :acknowledged)
end

# A missing native extension is an installation error, not a Ruby fallback.
# It builds Status values, so the Data class above must exist first.
require 'gossip_machines_native/gossip_machines_native'
