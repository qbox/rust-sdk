# frozen_string_literal: true

require 'ffi'

module QiniuNg
  class Config
    DEFAULT_APPENDED_USER_AGENT = ["qiniu-ruby", VERSION, RUBY_ENGINE, RUBY_ENGINE_VERSION, RUBY_PLATFORM].freeze

    def initialize(use_https: nil,
                   api_host: nil,
                   rs_host: nil,
                   rsf_host: nil,
                   uc_host: nil,
                   uplog_host: nil,
                   batch_max_operation_size: nil,
                   http_connect_timeout: nil,
                   http_low_transfer_speed: nil,
                   http_low_transfer_speed_timeout: nil,
                   http_request_retries: nil,
                   http_request_retry_delay: nil,
                   http_request_timeout: nil,
                   tcp_keepalive_idle_timeout: nil,
                   tcp_keepalive_probe_interval: nil,
                   upload_block_size: nil,
                   upload_threshold: nil,
                   upload_token_lifetime: nil,
                   upload_recorder_always_flush_records: nil,
                   upload_recorder_root_directory: nil,
                   upload_recorder_upload_block_lifetime: nil,
                   builder: nil)
      builder ||= Builder.new
      builder.use_https = use_https unless use_https.nil?
      builder.api_host = api_host unless api_host.nil?
      builder.rs_host = rs_host unless rs_host.nil?
      builder.rsf_host = rsf_host unless rsf_host.nil?
      builder.uc_host = uc_host unless uc_host.nil?
      builder.uplog_host = uplog_host unless uplog_host.nil?
      builder.batch_max_operation_size = batch_max_operation_size unless batch_max_operation_size.nil?
      builder.http_connect_timeout = http_connect_timeout unless http_connect_timeout.nil?
      builder.http_low_transfer_speed = http_low_transfer_speed unless http_low_transfer_speed.nil?
      builder.http_low_transfer_speed_timeout = http_low_transfer_speed_timeout unless http_low_transfer_speed_timeout.nil?
      builder.http_request_retries = http_request_retries unless http_request_retries.nil?
      builder.http_request_retry_delay = http_request_retry_delay unless http_request_retry_delay.nil?
      builder.http_request_timeout = http_request_timeout unless http_request_timeout.nil?
      builder.tcp_keepalive_idle_timeout = tcp_keepalive_idle_timeout unless tcp_keepalive_idle_timeout.nil?
      builder.tcp_keepalive_probe_interval = tcp_keepalive_probe_interval unless tcp_keepalive_probe_interval.nil?
      builder.upload_block_size = upload_block_size unless upload_block_size.nil?
      builder.upload_threshold = upload_threshold unless upload_threshold.nil?
      builder.upload_token_lifetime = upload_token_lifetime unless upload_token_lifetime.nil?
      builder.upload_recorder_always_flush_records = upload_recorder_always_flush_records unless upload_recorder_always_flush_records.nil?
      builder.upload_recorder_root_directory = upload_recorder_root_directory unless upload_recorder_root_directory.nil?
      builder.upload_recorder_upload_block_lifetime = upload_recorder_upload_block_lifetime unless upload_recorder_upload_block_lifetime.nil?
      @config = QiniuNg::Error.wrap_ffi_function do
                  Bindings::Config.build(builder.instance_variable_get(:@builder))
                end
    end

    # 设置布尔值属性 Getters
    %i[use_https
       domains_manager_auto_persistent_disabled
       domains_manager_url_resolution_disabled
       upload_recorder_always_flush_records
       uplog_enabled].each do |method|
      define_method(:"#{method}?") do
        @config.public_send(:"get_#{method}")
      end
    end

    # 设置整型属性 Getters
    %i[batch_max_operation_size
       domains_manager_url_resolve_retries
       http_request_retries
       http_low_transfer_speed
       upload_block_size
       upload_threshold].each do |method|
      define_method(method) do
        @config.public_send(:"get_#{method}")
      end
    end

    # 设置字符串属性 Getters
    %i[api_host
       api_url
       rs_host
       rs_url
       rsf_host
       rsf_url
       uc_host
       uc_url
       uplog_host
       uplog_url
       user_agent
       upload_recorder_root_directory
       uplog_file_path].each do |method|
      define_method(method) do
        cache = instance_variable_get(:"@#{method}")
        if cache.nil?
          cache = @config.public_send(:"get_#{method}")
          instance_variable_set(:"@#{method}", cache)
        end
        cache&.get_ptr
      end
    end

    # 设置时间型属性 Getters
    %i[domains_manager_auto_persistent_interval
       domains_manager_resolutions_cache_lifetime
       domains_manager_url_frozen_duration
       domains_manager_url_resolve_retry_delay
       http_connect_timeout
       http_low_transfer_speed_timeout
       http_request_retry_delay
       http_request_timeout
       tcp_keepalive_idle_timeout
       tcp_keepalive_probe_interval
       upload_recorder_upload_block_lifetime
       upload_token_lifetime].each do |method|
      define_method(method) do
        Utils::Duration.new(seconds: @config.public_send(:"get_#{method}"))
      end
    end

    module TempStruct
      class LockPolicy < FFI::Struct
        core_ffi = Bindings.const_get(:CoreFFI)
        layout :enum, core_ffi::QiniuNgUploadLoggerLockPolicyT
      end

      class U32 < FFI::Struct
        layout :value, :uint32
      end
    end
    private_constant :TempStruct

    def uplog_file_lock_policy
      policy = TempStruct::LockPolicy.new
      return nil unless @config.get_uplog_file_lock_policy(policy)
      policy[:enum]
    end

    def uplog_file_upload_threshold
      u32 = TempStruct::U32.new
      return nil unless @config.get_uplog_file_upload_threshold(u32)
      u32[:value]
    end

    def uplog_file_max_size
      u32 = TempStruct::U32.new
      return nil unless @config.get_uplog_file_max_size(u32)
      u32[:value]
    end

    class Builder
      def initialize
        @builder = self.class.send(:new_default)
      end

      def build!
        Config.new(builder: self)
      ensure
        @builder = self.class.send(:new_default)
      end

      def self.new_default
        Bindings::ConfigBuilder.new!.tap do |builder|
          builder.set_appended_user_agent(DEFAULT_APPENDED_USER_AGENT.join('/'))
        end
      end
      private_class_method :new_default

      # 设置无参数 Setters
      %i[enable_uplog
         disable_uplog
         domains_manager_sync_pre_resolve
         domains_manager_async_pre_resolve
         domains_manager_disable_auto_persistent
         domains_manager_disable_url_resolution
         domains_manager_enable_url_resolution].each do |method|
        define_method(method) do
          @builder.public_send(method)
        end
      end

      # 设置布尔型参数 Setters
      %i[use_https
         upload_recorder_always_flush_records].each do |method|
        define_method(method) do |arg|
          @builder.public_send(method, !!arg)
        end
        alias_method :"#{method}=", method
      end

      # 设置枚举型参数 Setters
      %i[uplog_file_lock_policy].each do |method|
        define_method(method) do |arg|
          @builder.public_send(method, arg.to_sym)
        end
        alias_method :"#{method}=", method
      end

      # 设置字符串属性 Setters
      %i[create_new_domains_manager
         load_domains_manager_from_file].each do |method|
        define_method(method) do |arg|
          @builder.public_send(method, arg.to_s)
        end
      end

      def set_appended_user_agent(user_agent)
        return ArgumentError, 'argument must not be nil' if user_agent.nil?
        user_agent = [user_agent.to_s] unless user_agent.is_a?(Array)
        user_agent = (DEFAULT_APPENDED_USER_AGENT + user_agent).join('/')
        @builder.set_appended_user_agent(user_agent)
      end

      %i[api_host
         rs_host
         rsf_host
         uc_host
         uplog_host
         domains_manager_persistent_file_path
         domains_manager_pre_resolve_url
         upload_recorder_root_directory
         uplog_file_path].each do |method|
        define_method(method) do |arg|
          @builder.public_send(method, arg.to_s)
        end
        alias_method :"#{method}=", method
      end

      # 设置整型和时间型属性 Setters
      [[:batch_max_operation_size, 0, 1 << 32 - 1],
       [:domains_manager_auto_persistent_interval, 0, 1 << 64 - 1],
       [:domains_manager_resolutions_cache_lifetime, 0, 1 << 64 - 1],
       [:domains_manager_url_frozen_duration, 0, 1 << 64 - 1],
       [:domains_manager_url_resolve_retries, 0, 1 << 32 - 1],
       [:domains_manager_url_resolve_retry_delay, 0, 1 << 64 - 1],
       [:http_connect_timeout, 0, 1 << 64 - 1],
       [:http_low_transfer_speed, 0, 1 << 32 - 1],
       [:http_low_transfer_speed_timeout, 0, 1 << 64 - 1],
       [:http_request_retries, 0, 1 << 32 - 1],
       [:http_request_retry_delay, 0, 1 << 64 - 1],
       [:http_request_timeout, 0, 1 << 64 - 1],
       [:tcp_keepalive_idle_timeout, 0, 1 << 64 - 1],
       [:tcp_keepalive_probe_interval, 0, 1 << 64 - 1],
       [:upload_block_size, 0, 1 << 32 - 1],
       [:upload_recorder_upload_block_lifetime, 0, 1 << 64 - 1],
       [:upload_threshold, 0, 1 << 32 - 1],
       [:upload_token_lifetime, 0, 1 << 64 - 1],
       [:uplog_file_max_size, 0, 1 << 32 - 1],
       [:uplog_file_upload_threshold, 0, 1 << 32 - 1]].each do |method, min_value, max_value|
        define_method(method) do |arg|
          arg = arg.to_i
          raise RangeError, "#{arg} is out of range" if arg > max_value || arg < min_value
          @builder.public_send(method, arg)
        end
        alias_method :"#{method}=", method
      end
    end
  end
end
