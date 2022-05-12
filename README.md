# Rustable

Rustable is an authorization server for [Medusa](https://github.com/Medusa-Team/linux-medusa) security module. Its primary focus is to address the disadvantages of existing implementations, specifically [Constable](https://github.com/Medusa-Team/Constable) and [mYstable](https://github.com/Medusa-Team/mYstable), using the benefits of [Rust](https://www.rust-lang.org/).

## Usage

The project serves as a library and is not available on [crates.io](https://crates.io/) currently. The only way to use this library is by cloning the repository and adding a local dependency to `Cargo.toml`.

Some examples are also provided with this repository. For example, to run `sshd` configuration, use:
```
$ cargo run --example sshd
```
