#![doc = include_str!("../README.md")]

use std::ffi::{c_char, c_void, CString};
use std::io::Write;
use std::ptr::NonNull;

#[derive(Debug)]
pub struct FfiLogger {
    data: Option<NonNull<c_void>>,
    logger: extern "C" fn(Option<NonNull<c_void>>, *const c_char) -> isize,
}

unsafe impl Send for FfiLogger {}

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
    /// Then it is safe to do any deallocation on the FFI side.
    /// 
    /// # Safety
    /// * The callback & data must be safe to be used across different threads.
    /// * Once [log::set_max_level] is set to [log::LevelFilter::Off], Rust code must not be called into again.
    pub unsafe fn new(
        logger: extern "C" fn(Option<NonNull<c_void>>, *const c_char) -> isize,
        data: Option<NonNull<c_void>>,
    ) -> FfiLogger {
        Self { logger, data }
    }

    pub fn into_data(self) -> Option<NonNull<c_void>> {
        let Self { data, .. } = self;

        data
    }
}

impl Write for FfiLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let c_str = CString::new(buf)?;

        let written = (self.logger)(self.data, c_str.as_ptr());

        match written.try_into() {
            // If it suceeds, that means some non-negative value was returned.
            Ok(written) => Ok(written),

            // If it fails, then it is negative so provide the error code.
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("FFI logging error: {written}"),
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
