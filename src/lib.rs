#[macro_use]
extern crate lazy_static;

pub mod medusa;

pub fn cstr_to_string(name: &[u8]) -> String {
    let vec = name
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&vec).into_owned()
}
