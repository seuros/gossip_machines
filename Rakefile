# frozen_string_literal: true

require 'rake/testtask'
require 'rb_sys/extensiontask'
require 'rubygems/package'

GEMSPEC = Gem::Specification.load('gossip_machines.gemspec')

SUPPORTED_NATIVE_PLATFORMS = %w[
  arm64-darwin
  x86_64-darwin
  aarch64-linux
  x86_64-linux
  x86_64-linux-musl
].freeze

native_ext = RbSys::ExtensionTask.new('gossip_machines_native', GEMSPEC) do |extension|
  extension.lib_dir = 'lib/gossip_machines_native'
  extension.tmp_dir = 'tmp/rb_sys'
  extension.cross_platform = SUPPORTED_NATIVE_PLATFORMS if ENV.key?('RUBY_TARGET')
end

# A single-version cross build stages its binary at the host binary's path,
# which the gem package depends on, so packaging would also compile for the
# host and fail where the host linker can't target the platform.
if ENV.key?('RUBY_TARGET')
  host_binary = "#{native_ext.lib_dir}/#{native_ext.name}.#{RbConfig::CONFIG['DLEXT']}"
  Rake::Task[host_binary].clear_prerequisites if Rake::Task.task_defined?(host_binary)
end

Rake::TestTask.new(:ruby_test) do |task|
  task.libs << 'lib'
  task.libs << 'test'
  task.pattern = 'test/**/*_test.rb'
end

desc 'Compile the native extension and test Ruby boundaries'
task test: %i[compile ruby_test]

desc 'Validate Ruby type signatures'
task :rbs do
  sh 'rbs', '-I', 'sig', 'validate'
end

desc 'Build a local source gem (no release or publication tasks)'
task :build do
  mkdir_p 'pkg'
  Gem::Package.build(GEMSPEC, false, false, "pkg/gossip_machines-#{GEMSPEC.version}.gem")
end

desc 'Test the Rust-only default member'
task :cargo do
  sh 'cargo', 'test'
end

task default: %i[test cargo rbs]

Dir[File.join(__dir__, 'tasks/*.rake')].sort.each { |f| load f }
