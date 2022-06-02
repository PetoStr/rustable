use anyhow::Result;
use rustable::medusa::{
    Config, ConfigError, Connection, Context, HandlerArgs, HandlerFlags, MedusaAnswer, SpaceBuilder,
};
use rustable_codegen::handler;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

#[handler(subject_vs = "*", event = "getprocess", object_vs = "*")]
async fn getprocess_handler(ctx: &Context, args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    let evtype = args.evtype;
    let mut subject = args.subject;

    subject.enter_tree(ctx, &evtype, "domains", "/").await;

    Ok(MedusaAnswer::Allow)
}

#[handler(subject_vs = "all_domains", event = "mkdir", object_vs = "all_files")]
async fn mkdir_handler(_ctx: &Context, _args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    Ok(MedusaAnswer::Allow)
}

#[handler(subject_vs = "all_domains", event = "rmdir", object_vs = "all_files")]
async fn rmdir_handler(_ctx: &Context, _args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    Ok(MedusaAnswer::Allow)
}

#[rustfmt::skip]
fn create_config() -> Result<Config, ConfigError> {
    let all_files = SpaceBuilder::new()
        .with_name("all_files")
        .with_path_recursive("fs/");

    let all_domains = SpaceBuilder::new()
        .with_name("all_domains")
        .with_path_recursive("domains/")
        .reads(["all_files", "all_domains"])
        .writes(["all_files", "all_domains"])
        .sees(["all_files", "all_domains"]);

    Config::builder()
        .add_space(all_files)
        .add_space(all_domains)

        .add_hierarchy_event_handler("getfile", "fs", Some("filename"), HandlerFlags::FROM_OBJECT)
        .add_custom_event_handler(getprocess_handler)
        .add_custom_event_handler(mkdir_handler)
        .add_custom_event_handler(rmdir_handler)
        .build()
}

#[tokio::main]
async fn main() -> Result<()> {
    use anyhow::Context;
    let config = create_config().context("Failed to create config")?;

    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)?;
    let read_handle = write_handle.try_clone()?;

    let mut connection = Connection::new(write_handle, read_handle, config)
        .await
        .context("Connection failed")?;
    connection.run().await.context("Communication failed")?;

    Ok(())
}
