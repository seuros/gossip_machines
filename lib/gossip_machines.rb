# frozen_string_literal: true

require_relative 'gossip_machines/version'

# A missing native extension is an installation error, not a Ruby fallback.
require 'gossip_machines_native/gossip_machines_native'
