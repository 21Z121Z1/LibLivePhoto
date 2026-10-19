//! Isolated foreign pointer/ABI boundary. The Core, Formats, operation interface
//! and WASM bridge retain their unsafe-code prohibition. No foreign pointers or
//! JNI objects escape a call; Rust owns responses until the matching free call.
#![deny(unsafe_op_in_unsafe_fn)]
use liblivephoto_interface::{process, OperationResult, MAX_INPUT};
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
};
struct Response(Vec<u8>);
fn failure(code: &str, message: &str) -> Vec<u8> {
    OperationResult::failure(code, message)
        .into_envelope()
        .expect("fixed error response fits protocol budget")
}
fn execute(request: &[u8], primary: &[u8], movie: &[u8]) -> Vec<u8> {
    match catch_unwind(AssertUnwindSafe(|| {
        let request = std::str::from_utf8(request).map_err(|_| "request is not UTF-8")?;
        process(request, primary, movie).into_envelope()
    })) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => failure("nativeError", &e),
        Err(_) => failure(
            "nativePanic",
            "native operation panicked; no output was published",
        ),
    }
}
#[no_mangle]
pub extern "C" fn llp_api_version() -> u32 {
    1
}
/// # Safety
/// Non-null inputs must point to immutable readable buffers of their stated
/// lengths for this call. A null pointer is accepted only when its length is 0.
/// The returned opaque handle must be freed exactly once with llp_response_free.
#[no_mangle]
pub unsafe extern "C" fn llp_process(
    request: *const u8,
    request_len: usize,
    primary: *const u8,
    primary_len: usize,
    movie: *const u8,
    movie_len: usize,
) -> *mut c_void {
    let bytes = if request_len > 1024 * 1024
        || primary_len
            .checked_add(movie_len)
            .is_none_or(|n| n > MAX_INPUT)
    {
        failure("inputLimit", "native input budget exceeded")
    } else if (request.is_null() && request_len > 0)
        || (primary.is_null() && primary_len > 0)
        || (movie.is_null() && movie_len > 0)
    {
        failure("invalidPointer", "null pointer with nonzero input length")
    } else {
        // SAFETY: The foreign caller contract above guarantees readable input spans;
        // zero-length spans use a real Rust empty slice instead of a null raw slice.
        let req = if request_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(request, request_len) }
        };
        let image = if primary_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(primary, primary_len) }
        };
        let video = if movie_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(movie, movie_len) }
        };
        execute(req, image, video)
    };
    Box::into_raw(Box::new(Response(bytes))).cast()
}
/// # Safety
/// response must be null or a live handle returned by llp_process. The returned
/// bytes remain valid and immutable until that handle is freed.
#[no_mangle]
pub unsafe extern "C" fn llp_response_data(response: *const c_void) -> *const u8 {
    if response.is_null() {
        std::ptr::null()
    } else {
        unsafe { &*response.cast::<Response>() }.0.as_ptr()
    }
}
/// # Safety
/// response must be null or a live handle returned by llp_process.
#[no_mangle]
pub unsafe extern "C" fn llp_response_len(response: *const c_void) -> usize {
    if response.is_null() {
        0
    } else {
        unsafe { &*response.cast::<Response>() }.0.len()
    }
}
/// # Safety
/// response must be null or an unfreed handle returned by llp_process. No spans
/// obtained from it may be used afterward, and it must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn llp_response_free(response: *mut c_void) {
    if !response.is_null() {
        drop(unsafe { Box::from_raw(response.cast::<Response>()) });
    }
}

use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jbyteArray,
    JNIEnv,
};
#[no_mangle]
pub extern "system" fn Java_org_liblivephoto_NativeEngine_nativeProcess(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    request: JString<'_>,
    primary: JByteArray<'_>,
    movie: JByteArray<'_>,
) -> jbyteArray {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<Vec<u8>, String> {
        let first = env.get_array_length(&primary).map_err(|e| e.to_string())? as usize;
        let second = env.get_array_length(&movie).map_err(|e| e.to_string())? as usize;
        if first.checked_add(second).is_none_or(|n| n > MAX_INPUT) {
            return Ok(failure(
                "inputLimit",
                "JNI byte-array input budget exceeded",
            ));
        }
        let chars = env
            .call_method(&request, "length", "()I", &[])
            .and_then(|v| v.i())
            .map_err(|e| e.to_string())?;
        if chars < 0 || chars as usize > 1024 * 1024 {
            return Ok(failure(
                "inputLimit",
                "JNI request character budget exceeded",
            ));
        }
        let request: String = env.get_string(&request).map_err(|e| e.to_string())?.into();
        if request.len() > 1024 * 1024 {
            return Ok(failure("inputLimit", "JNI request budget exceeded"));
        }
        let primary = env
            .convert_byte_array(&primary)
            .map_err(|e| e.to_string())?;
        let movie = env.convert_byte_array(&movie).map_err(|e| e.to_string())?;
        Ok(execute(request.as_bytes(), &primary, &movie))
    }));
    let bytes = match result {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => failure("jniError", &e),
        Err(_) => failure("nativePanic", "JNI operation panicked"),
    };
    // Preserve an existing JVM exception (including OOM); do not replace it with
    // a fabricated success response or clear it. JNI locals die with this call.
    if env.exception_check().unwrap_or(true) {
        return std::ptr::null_mut();
    }
    env.byte_array_from_slice(&bytes)
        .map(|b| b.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_abi_handles_valid_and_null_inputs_without_leaking_owned_response() {
        let request = br#"{"action":"version"}"#;
        unsafe {
            let handle = llp_process(
                request.as_ptr(),
                request.len(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
            );
            let data =
                std::slice::from_raw_parts(llp_response_data(handle), llp_response_len(handle));
            assert!(data.starts_with(b"LLP1"));
            llp_response_free(handle);
            let bad = llp_process(
                std::ptr::null(),
                3,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
            );
            assert!(!bad.is_null());
            llp_response_free(bad);
            llp_response_free(std::ptr::null_mut());
        }
    }
}
