# ffi-logger

This library is meant to be used in Rust libraries to expose the Rust logging interface to a C FFI application, so that the application can integrate it into its own logging system. It is made to be used with `env_logger` but may be able to be used with other logging implementations.

# Example
Some library that is exposing some API to a C application.

```rust
use std::ffi::c_char;
use std::ffi::c_void;
use std::ptr::NonNull;

use ffi_logger::{FfiLogger, LogHandle};
use log::{info, LevelFilter};

/// Library state for incrementing a counter.
pub struct inc_State {
    pub(crate) state: u32,
    pub(crate) log_handle: LogHandle
}

/// Library init function
///
/// # Safety
/// The data & function pair provided must be thread-safe.
///
/// The logger function must flush the logging data on each call.
///
/// The data must live until the [inc_deinit] funtion is called.
#[no_mangle]
pub unsafe extern "C" fn inc_init(
    start: u32,
    logger: extern "C" fn(Option<NonNull<c_void>>, log::Level, *const c_char),
    data: Option<NonNull<c_void>>,
) -> Option<Box<inc_State>> {
    let log_handle = Box::leak(Box::new(FfiLogger::new(
        logger,
        data
    )));

    if log::set_logger(log_handle).is_err() {
        return None;
    }

    Some(Box::new(inc_State { state: start, log_handle: LogHandle::new(log_handle) }))
}

/// Increments the library state
#[no_mangle]
pub extern "C" fn inc_increment(state: &mut inc_State) {
    state.state = state.state + 1;

    info!("incremented, new value {}", state.state);
}

#[no_mangle]
pub extern "C" fn inc_deinit(state: Box<inc_State>) -> Option<NonNull<c_void>> {
    log::set_max_level(LevelFilter::Off);
    state.log_handle.deinit()
}

```

Then on the C side

```c
// libinc.h
#include <unistd.h>
#include <stdint.h>

typedef struct inc_State inc_State;

inc_State* inc_init(
    int32_t start,
    void(*logger)(void*, size_t, const char*),
    void* data
);

void inc_increment(inc_State*);

void* inc_deinit(inc_State*);

```

```c
#include <stdint.h>
#include <stdio.h>
#include <errno.h>
#include <unistd.h>

#include "libinc.h"

// Callback for logging
void write_log(FILE* log_file, size_t log_level, char* to_write) {
    switch (log_level) {
        case 1:
            fputs("ERROR: ", log_file)
            break;
        case 2:
            fputs("WARNING: ", log_file)
            break;
        case 3:
            fputs("INFO: ", log_file)
            break;
        case 4:
            fputs("DEBUG: ", log_file)
            break;
        case 5:
            fputs("TRACE: ", log_file)
            break;
        default:
            fputs("UNKNOWN: ", log_file)
            break;

    }

    fputs(to_write, log_file);
    fflush(log_file);
}

int main() {
    FILE* log_file = fopen("some_file.log", "a");
    assert(file);

    inc_State* library_state = inc_init(0, write_log, log_file);

    // The FFI side can still use the logger as well.
    if (fputs("Start of increment\n", log_file) < 0) {
        clearerr(log_file);
    }
    inc_increment(library_state);
    inc_increment(library_state);
    inc_increment(library_state);
    if (fputs("Time to deinit\n", log_file) < 0) {
        clearerr(log_file);
    }

    inc_deinit(library_state);

    return fclose(log_file);
}

```
