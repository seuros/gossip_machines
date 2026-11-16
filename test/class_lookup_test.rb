# frozen_string_literal: true

require 'minitest/autorun'
require 'open3'
require 'rbconfig'

# The class of the value the extension wraps (Rumor) is fixed when it loads.
# Nothing a caller does later, redefining the constant or
# `Object.const_get`, reaches a rumor being built. Each case runs in a fresh
# process: the class is fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'gossip_machines'
    G = GossipMachines
    Rumor = G::Rumor
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: a Rumor constant redefined before the first rumor made it an
  # instance of the new class, which has none of the rumor's methods.
  def test_a_redefined_rumor_constant_does_not_change_the_class
    assert_equal 'true 7 acknowledged true', isolated(<<~RUBY)
      G.send(:remove_const, :Rumor)
      G.const_set(:Rumor, Class.new)
      rumor = Rumor.new([7], 1)
      puts [rumor.instance_of?(Rumor), rumor.round(0, 1).join, rumor.acknowledge(7), rumor.delivered?].join(' ')
    RUBY
  end

  # Bug: building the first rumor ran `Object.const_get`, Ruby code in the
  # middle of a native call. No Ruby runs while one is built now.
  def test_building_a_rumor_runs_no_const_get
    assert_equal '0 Rumor', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      rumor = Rumor.new([7], 1)
      puts [hits, rumor.class.name.split('::').last].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal '7 acknowledged', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran in a native call' }
      rumor = Rumor.new([7], 1)
      puts [rumor.round(0, 1).join, rumor.acknowledge(7)].join(' ')
    RUBY
  end
end
