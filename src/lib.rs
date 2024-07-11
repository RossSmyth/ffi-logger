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
    /// # Safety
    /// The data must be able to be sent between threads. That means that it must either be
    /// [Send], or the logger function must take a lock of some sort.
    ///
    /// The data also must be held exclusively by the [FfiLogger] struct until the logger is deinitialized.
    ///
    /// The data must also be flushed on ever call to the logger function.
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
