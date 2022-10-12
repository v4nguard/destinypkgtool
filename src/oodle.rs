use std::ptr::null_mut;

use libc::c_void;

#[link(name = "oo2core_3")]
extern "C" {
    pub fn OodleLZ_Decompress(
        buffer: *const u8,
        buffer_size: i64,
        output_buffer: *mut u8,
        output_buffer_size: i64,
        a: i32,
        b: i32,
        c: i64,
        d: *mut c_void,
        e: *mut c_void,
        f: *mut c_void,
        g: *mut c_void,
        h: *mut c_void,
        i: *const c_void,
        thread_module: i32,
    ) -> i64;
}

pub fn decompress(buffer: &[u8], output_buffer: &mut [u8]) -> i64 {
    unsafe {
        OodleLZ_Decompress(
            buffer.as_ptr() as *mut u8,
            buffer.len() as i64,
            output_buffer.as_mut_ptr(),
            output_buffer.len() as i64,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            3,
        )
    }
}
