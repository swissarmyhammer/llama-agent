2025-08-12T15:37:15.772774Z DEBUG llama_agent::queue: Tokenized prompt to 2641 tokens
/Users/wballard/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/llama-cpp-sys-2-0.1.116/llama.cpp/src/llama-context.cpp:985: GGML_ASSERT(n_tokens_all <= cparams.n_batch) failed
(lldb) process attach --pid 57881
Process 57881 stopped
* thread #1, name = 'main', queue = 'com.apple.main-thread', stop reason = signal SIGSTOP
    frame #0: 0x00000001810cc3cc libsystem_kernel.dylib`__psynch_cvwait + 8
libsystem_kernel.dylib`__psynch_cvwait:
->  0x1810cc3cc <+8>:  b.lo   0x1810cc3ec    ; <+40>
    0x1810cc3d0 <+12>: pacibsp
    0x1810cc3d4 <+16>: stp    x29, x30, [sp, #-0x10]!
    0x1810cc3d8 <+20>: mov    x29, sp
Target 0: (basic_usage) stopped.
Executable binary set to "/Users/wballard/github/llama-agent/target/debug/examples/basic_usage".
Architecture set to: arm64-apple-macosx-.
(lldb) bt
* thread #1, name = 'main', queue = 'com.apple.main-thread', stop reason = signal SIGSTOP
  * frame #0: 0x00000001810cc3cc libsystem_kernel.dylib`__psynch_cvwait + 8
    frame #1: 0x000000018110b0e0 libsystem_pthread.dylib`_pthread_cond_wait + 984
    frame #2: 0x000000010088a1dc basic_usage`parking_lot::condvar::Condvar::wait_until_internal::h07038009a29e47d3 [inlined] _$LT$parking_lot_core..thread_parker..imp..ThreadParker$u20$as$u20$parking_lot_core..thread_parker..ThreadParkerT$GT$::park::he38ec8d7f178d263(self=<unavailable>) at unix.rs:77:21 [opt]
    frame #3: 0x000000010088a1b8 basic_usage`parking_lot::condvar::Condvar::wait_until_internal::h07038009a29e47d3 [inlined] parking_lot_core::parking_lot::park::_$u7b$$u7b$closure$u7d$$u7d$::hb6afe0206944f719(thread_data=0x0000000138e17e88) at parking_lot.rs:635:17 [opt]
    frame #4: 0x000000010088a054 basic_usage`parking_lot::condvar::Condvar::wait_until_internal::h07038009a29e47d3 [inlined] parking_lot_core::parking_lot::with_thread_data::h0ad85d35afc415ab(f=<unavailable>) at parking_lot.rs:207:5 [opt]
    frame #5: 0x0000000100889fc0 basic_usage`parking_lot::condvar::Condvar::wait_until_internal::h07038009a29e47d3 [inlined] parking_lot_core::parking_lot::park::hba5ce86d4c10b90c(key=<unavailable>, validate=<unavailable>, before_sleep=<unavailable>, timed_out=<unavailable>, park_token=(__0 = 0), timeout=<unavailable>) at parking_lot.rs:600:5 [opt]
    frame #6: 0x0000000100889fc0 basic_usage`parking_lot::condvar::Condvar::wait_until_internal::h07038009a29e47d3(self=0x0000000138e1cc78, mutex=0x0000000138e1cc80, timeout=Option<std::time::Instant> @ 0x0000000387aa69a0) at condvar.rs:334:17 [opt]
    frame #7: 0x000000010087c970 basic_usage`std::io::error::Error::kind::hf7114431e4af958e(self=<unavailable>) at error.rs:993:9 [opt]
    frame #8: 0x00000001003f934c basic_usage`tokio::runtime::park::CachedParkThread::block_on::h1c76f0dc9a62b30f(self=0x0000000000000001, f={async_block_env#0} @ 0x0000000138e1cc80) at park.rs:289:13 [opt]
    frame #9: 0x00000001003fc33c basic_usage`tokio::runtime::context::runtime::enter_runtime::h7f3786e398cc9fd2 [inlined] tokio::runtime::context::blocking::BlockingRegionGuard::block_on::h049d4d6040308c4e(self=0x000000016fa052a0, f=<unavailable>) at blocking.rs:66:9 [opt]
    frame #10: 0x00000001003fc32c basic_usage`tokio::runtime::context::runtime::enter_runtime::h7f3786e398cc9fd2 [inlined] tokio::runtime::scheduler::multi_thread::MultiThread::block_on::_$u7b$$u7b$closure$u7d$$u7d$::h132cc83e93f92ed7(blocking=0x000000016fa052a0) at mod.rs:87:13 [opt]
    frame #11: 0x00000001003fc32c basic_usage`tokio::runtime::context::runtime::enter_runtime::h7f3786e398cc9fd2(handle=<unavailable>, allow_block_in_place=true, f={closure_env#0}<basic_usage::main::{async_block_env#0}> @ 0x000000016fa05680) at runtime.rs:65:16 [opt]
    frame #12: 0x0000000100404524 basic_usage`tokio::runtime::runtime::Runtime::block_on::hf5afcb5c554b0ea2 [inlined] tokio::runtime::scheduler::multi_thread::MultiThread::block_on::h60ec5402a3e0e63b(self=0x000000016fa05a08, handle=<unavailable>, future=<unavailable>) at mod.rs:86:9 [opt]
    frame #13: 0x0000000100404518 basic_usage`tokio::runtime::runtime::Runtime::block_on::hf5afcb5c554b0ea2 [inlined] tokio::runtime::runtime::Runtime::block_on_inner::h6610397128f676ad(self=0x000000016fa05a00, future={async_block_env#0} @ 0x000000016fa054b0, (null)=<unavailable>) at runtime.rs:358:45 [opt]
    frame #14: 0x00000001004044e8 basic_usage`tokio::runtime::runtime::Runtime::block_on::hf5afcb5c554b0ea2(self=0x000000016fa05a00, future=<unavailable>) at runtime.rs:330:13 [opt]
    frame #15: 0x00000001003fdc70 basic_usage`basic_usage::main::h74191c5a0c561d80 at basic_usage.rs:170:5 [opt]
    frame #16: 0x0000000100400d00 basic_usage`std::sys::backtrace::__rust_begin_short_backtrace::h7a359b27d7d2c263 [inlined] core::ops::function::FnOnce::call_once::h05d1ce1dadb4842f((null)=<unavailable>, (null)=<unavailable>) at function.rs:250:5 [opt]
    frame #17: 0x0000000100400cfc basic_usage`std::sys::backtrace::__rust_begin_short_backtrace::h7a359b27d7d2c263(f=<unavailable>) at backtrace.rs:152:18 [opt]
    frame #18: 0x00000001003fdd8c basic_usage`std::rt::lang_start::h084b1e3eec30c468(main=<unavailable>, argc=<unavailable>, argv=<unavailable>, sigpipe=<unavailable>) - 18446744069410398835 [opt]
    frame #19: 0x00000001008aac1c basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] core::ops::function::impls::_$LT$impl$u20$core..ops..function..FnOnce$LT$A$GT$$u20$for$u20$$RF$F$GT$::call_once::he7ba0572945420d1 at function.rs:284:13 [opt]
    frame #20: 0x00000001008aac14 basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panicking::try::do_call::hebe393b810f01e71 at panicking.rs:587:40 [opt]
    frame #21: 0x00000001008aac10 basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panicking::try::hb25fce0758ef422c at panicking.rs:550:19 [opt]
    frame #22: 0x00000001008aac10 basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panic::catch_unwind::h84fa9d32cc13223f at panic.rs:358:14 [opt]
    frame #23: 0x00000001008aac10 basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::rt::lang_start_internal::_$u7b$$u7b$closure$u7d$$u7d$::h303447aa1f5dac68 at rt.rs:168:24 [opt]
    frame #24: 0x00000001008aa8cc basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panicking::try::do_call::h6fee0bd35745e600 at panicking.rs:587:40 [opt]
    frame #25: 0x00000001008aa8cc basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panicking::try::h8a1ab658538ac4f7 at panicking.rs:550:19 [opt]
    frame #26: 0x00000001008aa8cc basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c [inlined] std::panic::catch_unwind::hb51538dca89efd17 at panic.rs:358:14 [opt]
    frame #27: 0x00000001008aa8cc basic_usage`std::rt::lang_start_internal::h95cf27b851151b9c at rt.rs:164:5 [opt]
    frame #28: 0x00000001003fdd68 basic_usage`main + 52
    frame #29: 0x0000000180d6ab98 dyld`start + 6076
(lldb) quit
zsh: abort      cargo run --example basic_usage

## Proposed Solution

The issue is caused by a mismatch in batch size handling between the context configuration and batch creation. The problem occurs when:

1. The basic_usage example configures a `batch_size: 8192` in the `ModelConfig`
2. The prompt is tokenized to 2641 tokens (which is under 8192)
3. However, in the streaming path (line 826 in queue.rs), a hard-coded batch size of 512 is used
4. The context is created with `n_batch=8192` but the batch itself is created with size 512
5. When processing 2641 tokens with a 512-sized batch, the GGML assertion `n_tokens_all <= cparams.n_batch` fails

The solution is to:

1. **Fix inconsistent batch size usage**: Both streaming and batch processing should use `model_manager.get_batch_size()` consistently
2. **Add validation**: Ensure that the number of prompt tokens doesn't exceed the configured batch size
3. **Handle large prompts gracefully**: If a prompt exceeds batch size, either truncate or chunk it appropriately

### Implementation Steps:
1. Replace hard-coded 512 batch size on line 826 with `model_manager.get_batch_size()`
2. Add validation in both `process_batch_request_sync` and `process_streaming_request_sync` to check if `tokens_list.len() > batch_size`
3. If tokens exceed batch size, implement chunking or return an appropriate error message
4. Ensure the LlamaBatch creation uses the same batch size as the context configuration
