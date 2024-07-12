# ffi-logger

This library is meant to be used in Rust libraries to expose the Rust logging interface to a C FFI application, so that the application can integrate it into its own logging system. It is made to be used with `env_logger` but may be able to be used with other logging implementations.

# Example
Some library that is exposing some API to a C application.

```rust
use std::ffi::c_void;
use std::ptr::NonNull;
use std::ffi::c_char;

use log::{info, LevelFilter};
use env_logger::Target;
use ffi_logger::FfiLogger;

/// Library state for incrementing a counter.
pub struct inc_State {
    pub(crate) state: u32,
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
    logger: extern "C" fn(Option<NonNull<c_void>>, *const c_char) -> isize, 
    data: Option<NonNull<c_void>>
) -> Box<inc_State> {
    env_logger::builder().target(Target::Pipe(Box::new(FfiLogger::new(
            logger, data
        ))));
    Box::new(inc_State {
        state: start
    })
}

/// Increments the library state
#[no_mangle]
pub extern "C" fn inc_increment(state: &mut inc_State) {
    state.state = state.state + 1;

    info!("incremented, new value {}", state.state);
}

#[no_mangle]
pub extern "C" fn inc_deinit(state: Box<inc_State>) {
    log::set_max_level(LevelFilter::Off)
}

```

Then on the C side

```c
// libinc.h
#include <unistd.h>

typedef struct inc_State inc_State;

inc_State* inc_init(
    ssize_t(*logger)(void*, const char*),
    void* data
);

void inc_increment(inc_State*);

void inc_deinit(inc_State*);

```

```c
#include <stdint.h>
#include <stdio.h>
#include <errno.h>
#include <unistd.h>

#include "libinc.h"

// Callback for logging
ssize_t write_log(FILE* log_file, char* to_write) {
    errno = 0;

    size_t written = fwrite(to_write, 1, strlen(to_write), log_file);
    int err = fflush(log_file);
    if (err != 0) {
        clearerr(log_file);
        return -1 * errno;
    } else {
        return written;
    }
}

int main() {
    FILE* log_file = fopen("some_file.log", "a");
    assert(file);

    inc_State* library_state = inc_init(0, write_log, log_file);

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
