# RustFuture poll codes
UNIFFI_RUST_FUTURE_POLL_READY = 0
UNIFFI_RUST_FUTURE_POLL_WAKE = 1

# Handle map for storing write-end IO objects used by the continuation callbacks.
UNIFFI_ASYNC_HANDLE_MAP = UniffiHandleMap.new

# Continuation callback for async functions.
# Called by Rust when the future is ready to make progress.
# Writes the poll code to the pipe so the waiting thread/fiber can continue.
UNIFFI_CONTINUATION_CALLBACK = Proc.new do |data, poll_code|
  wr = UNIFFI_ASYNC_HANDLE_MAP.remove(data) rescue nil # never raise from FFI callback
  next unless wr # guard against concurrent cancellation cleanup already removing the handle
  wr.write([poll_code].pack('C'))
  wr.close
end

# Poll a Rust future to completion.
#
# This works both with and without a Fiber::Scheduler:
# - Without scheduler: wait_readable blocks with a timeout, releasing the GVL
#   so the callback can fire from foreign threads (works around MRI limitation
#   where rb_thread_call_with_gvl cannot wake up threads in indefinite sleep).
# - With scheduler: wait_readable hooks into io_wait, yielding the fiber.
#
# cancel_fn is called in the ensure block when exception interrupts an in-flight poll.
# This guarantees Rust fires the continuation callback so the handle-map entry is released
# and the pipe is drained before we free the future.
def self.uniffi_rust_call_async(rust_future, poll_fn, cancel_fn, complete_fn, free_fn, lift_func, error_ffi_converter)
  current_rd = nil
  current_handle = nil

  begin
    loop do
      rd, wr = IO.pipe
      handle = UNIFFI_ASYNC_HANDLE_MAP.insert(wr)
      current_rd = rd
      current_handle = handle
      UniFFILib.public_send(poll_fn, rust_future, UNIFFI_CONTINUATION_CALLBACK, handle)

      # wait_readable with a timeout ensures periodic thread wakeups and maps cleanly to
      # Fiber::Scheduler#io_wait for non-blocking fiber concurrency.
      nil until rd.wait_readable(0.01)
      poll_code = rd.read(1).unpack1('C')
      rd.close
      current_rd = nil
      current_handle = nil

      break if poll_code == UNIFFI_RUST_FUTURE_POLL_READY
    end

    result = if error_ffi_converter.nil?
      ::{{ ci.namespace()|class_name_rb }}.rust_call(complete_fn, rust_future)
    else
      ::{{ ci.namespace()|class_name_rb }}.rust_call_with_error(error_ffi_converter, complete_fn, rust_future)
    end

    lift_func.call(result)
  ensure
    if current_handle
      # An exception interrupted an in-flight poll. This prevents handle-map entry leaks.
      UniFFILib.public_send(cancel_fn, rust_future)
      # Wait up to 0.5s for the continuation to fire, then drain the pipe.
      current_rd.wait_readable(0.5) rescue nil
      current_rd.close rescue nil
      # Safety net: if the callback somehow never fired, remove the entry manually.
      if (leftover_wr = UNIFFI_ASYNC_HANDLE_MAP.remove(current_handle) rescue nil)
        leftover_wr.close rescue nil
      end
    end
    UniFFILib.public_send(free_fn, rust_future)
  end
end

{%- if ci.has_async_callback_interface_definition() %}
# Exception raised when a foreign future is canceled.
class UniffiInternalCancelled < RuntimeError; end

# User callback that raises it will be considered a Rust-side cancellation.
private_constant :UniffiInternalCancelled

# Handle map for storing Threads executing foreign async callbacks.
UNIFFI_FOREIGN_FUTURE_HANDLE_MAP = UniffiHandleMap.new

# One-shot claim flag: the first caller to `claim!` wins; all subsequent callers
# are no-ops. Used to enforce the at-most-once contract on uniffi_future_callback.
class UniffiOnceFlag
  def initialize
    @mutex = Mutex.new
    @claimed = false
  end

  # Returns true if this caller won the race (first to claim), false otherwise.
  def claim!
    @mutex.synchronize do
      first = !@claimed
      @claimed = true
      first
    end
  end
end

# Execute a foreign async callback method in a background thread.
# Enforces the at-most-once guarantee on handle_success / handle_error: whichever
# fires first (normal completion or Rust-side drop) suppresses the other.
def self.uniffi_trait_interface_call_async(make_call, uniffi_out_dropped_callback, handle_success, handle_error, error_type = nil, lower_error = nil)
  once = UniffiOnceFlag.new

  # Called by Rust when the foreign future is dropped (i.e. canceled).
  # Raises UniffiInternalCancelled in the worker thread so make_call can exit early,
  # but only if the thread hasn't already completed and claimed the once flag.
  dropped_callback = Proc.new do |handle|
    thread = UNIFFI_FOREIGN_FUTURE_HANDLE_MAP.remove handle
    thread.raise(UniffiInternalCancelled, 'Future was canceled') if once.claim! && thread&.alive?
  end

  thread = Thread.new do
    # Phase 1: run the user's async method.
    # UniffiInternalCancelled exits silently. Other exceptions are forwarded as errors.
    # handle_success is intentionally called outside this rescue so exceptions from it
    # cannot re-enter handle_error (which would be a double-call on the Rust sender).
    begin
      result = make_call.call
    rescue UniffiInternalCancelled
      next
    rescue StandardError => e
      next unless once.claim!
      if !error_type.nil? && ::{{ ci.namespace()|class_name_rb }}.uniffi_is_error_type?(e, error_type)
        handle_error.call(UNIFFI_CALLBACK_ERROR, lower_error.call(e))
      else
        handle_error.call(UNIFFI_CALLBACK_UNEXPECTED_ERROR, {{ "e.inspect"|lower_rb(&Type::String, config) }})
      end
      next
    end

    # Phase 2: deliver the result to Rust. Skipped if dropped_callback already fired.
    begin
      handle_success.call(result) if once.claim!
    rescue UniffiInternalCancelled
      # Thread#raise landed after Phase 1 - silently exit.
      next 
    end
  end

  handle = UNIFFI_FOREIGN_FUTURE_HANDLE_MAP.insert(thread)
  uniffi_out_dropped_callback[:handle] = handle
  uniffi_out_dropped_callback[:free] = dropped_callback
end
{%- endif %}
