# frozen_string_literal: true

require_relative 'lib/gossip_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'gossip_machines'
  spec.version = GossipMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']
  spec.summary = 'Native bounded rumor fanout decisions for Ruby'
  spec.description = 'A thin Ruby binding to the no_std gossip-machines Rust planner. ' \
                     'Tracks per-peer send attempts and acknowledgements without ' \
                     'owning payloads, transport or membership discovery.'
  spec.homepage = 'https://github.com/seuros/gossip_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'
  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'gossip_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/gossip_machines_native/ffi/Cargo.toml'

  # The core README is included by Rust at compile time.
  spec.files = Dir.glob(%w[
    lib/**/*.rb sig/**/*.rbs ext/**/*.{rb,rs,toml,md} ext/**/LICENSE
    Cargo.toml Cargo.lock README.md CHANGELOG.md LICENSE.txt
  ]).select { |path| File.file?(path) }
    .reject { |path| path.end_with?('/tests.rs') || path.include?('/tests/') }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/gossip_machines_native/extconf.rb']
  spec.add_dependency 'rb_sys', '~> 0.9', '>= 0.9.130'
end
