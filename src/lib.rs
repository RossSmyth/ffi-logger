#![doc = include_str!("../README.md")]

use std::ffi::{c_char, c_void, CString};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, Ordering};

use log::{log_enabled, Log};

type Callback = extern "C" fn(Option<NonNull<c_void>>, log::Level, *const c_char);

#[cfg(not(target_family = "wasm"))]
#[derive(Debug)]
pub struct FfiLogger {
    data: Option<NonNull<c_void>>,
    logger: AtomicPtr<()>,
}

// Safety: 
// It's just pointers. The user data needs to be thread safe.
unsafe impl Send for FfiLogger {}

// Safety:
// I swear to never ever mutate the data field.
// after construction. The logger field can be changed one time.
unsafe impl Sync for FfiLogger {}

impl FfiLogger {
    /// Create an instance of an FFI logger.
    ///
    /// This function is meant to be exposed by the FFI library in its own cohesive API. How that
    /// is is up to the library itself.
    ///
    /// The callback takes in an optional type-erased user data pointer, and a null-terminated
    /// string to be logged. The return value of the callback represents the number of bytes
    /// written if zero or larger, and if negative represents a user defined error code.
    ///
    /// The callback may be called from different threads, meaning that it may be used
    /// in parallel to the C FFI.
    ///
    /// Each call to the logger should flush the output so that each logged message is not
    /// interleaved.
    ///
    /// Once the Rust library is done, disable the logger with [log::set_max_level] to [log::LevelFilter::Off].
    /// Then the FFI side can deallocate or deinit whatever it needs.
    ///
    /// # Safety
    /// * The callback & data must be safe to be used across different threads.
    /// * Once [log::set_max_level] is set to [log::LevelFilter::Off], Rust code must not be called into again.
    pub unsafe fn new(
        logger: Callback,
        data: Option<NonNull<c_void>>,
    ) -> FfiLogger {
        Self { logger: AtomicPtr::new(logger as _), data }
    }
}

impl Log for FfiLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let log_fn = self.logger.load(Ordering::Relaxed);
        if log_fn.is_null() { return; }

        // Safety:
        // Function pointers have the same representation as data pointers
        // and we cfg'd out wasm.
        let log_fn: Callback = unsafe {std::mem::transmute(log_fn) };
        
        if log_enabled!(record.level()) {
            let message = match CString::new(record.args().to_string()) {
                Ok(cstr) => cstr,
                Err(err) => CString::new(
                    err.into_vec()
                        .into_iter()
                        .map(|char| if char == 0 { 0x1A } else { char })
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            };

            (log_fn)(self.data, record.level(), message.as_ptr());
        }
    }

    fn flush(&self) {}
}

pub struct LogHandle {
    logger: &'static FfiLogger,
}

impl LogHandle {
    pub fn new(logger: &'static FfiLogger) -> Self {
        Self {
            logger
        }
    }

    pub fn deinit(&self) -> Option<NonNull<c_void>> {
        self.logger.logger.store(ptr::null_mut(), Ordering::Relaxed);

        self.logger.data
    }
}
