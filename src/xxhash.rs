extern crate libc;

#[link(name="xxhash", kind="static")]
extern {
    fn XXH32(input: *const libc::c_void, length: libc::size_t, seed: libc::c_uint) -> libc::c_uint;
    fn XXH64(input: *const libc::c_void, length: libc::size_t, seed: libc::c_ulonglong) -> libc::c_ulonglong;
}

const SEED: u32 = 42;

pub fn xxhash32(input: &[u8], length: u64) -> u32 {
    unsafe {
        XXH32(&input[0] as *const _ as *const libc::c_void, length, SEED)
    }
}

pub fn xxhash64(input: &[u8], length: u64) -> u64 {
    unsafe {
        XXH64(&input[0] as *const _ as *const libc::c_void, length, SEED as u64)
    }
}
