//! Rustable is an implementation of authorization server for
//! [Medusa](https://github.com/Medusa-Team/linux-medusa) security module.
//!
//! # Example
//! ```
//! #[handler(subject_vs = "*", event = "getprocess", object_vs = "*")]
//! async fn getprocess_handler(ctx: &Context, args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
//!     let evtype = args.evtype;
//!     let mut subject = args.subject;
//!
//!     subject.enter_tree(ctx, &evtype, "domains", "/").await;
//!
//!     Ok(MedusaAnswer::Allow)
//! }
//!
//! fn create_config() -> Result<Config, ConfigError> {
//!     let all_files = SpaceBuilder::new()
//!         .with_name("all_files")
//!         .with_path_recursive("fs/");
//!
//!     let all_domains = SpaceBuilder::new()
//!         .with_name("all_domains")
//!         .with_path_recursive("domains/")
//!         .reads(["all_files", "all_domains"])
//!         .writes(["all_files", "all_domains"])
//!         .sees(["all_files", "all_domains"]);
//!
//!     Config::builder()
//!         .add_space(all_files)
//!         .add_space(all_domains)
//!
//!         .add_hierarchy_event_handler("getfile", "fs", Some("filename"), HandlerFlags::FROM_OBJECT)
//!         .add_custom_event_handler(getprocess_handler)
//!         .build()
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     use anyhow::Context;
//!     let config = create_config()?;
//!
//!     let write_handle = OpenOptions::new()
//!         .read(true)
//!         .write(true)
//!         .open("/dev/medusa")?;
//!     let read_handle = write_handle.try_clone()?;
//!
//!     let mut connection = Connection::new(write_handle, read_handle, config).await?;
//!     connection.run().await?;
//!
//!     Ok(())
//! }
//! ```

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
