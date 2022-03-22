//! Rustable is an implementation of authorization server for
//! [Medusa](https://github.com/Medusa-Team/linux-medusa) security module.

#[macro_use]
extern crate lazy_static;

pub mod bitmap;
pub mod medusa;

/// Converts null terminated bytes to [`std::string::String`].
pub fn cstr_to_string(cstr: &[u8]) -> String {
    let vec = cstr
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&vec).into_owned()
}
