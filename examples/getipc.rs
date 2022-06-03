use anyhow::Result;
use rustable::medusa::{
    AccessType, Config, ConfigError, Connection, Context, HandlerArgs, HandlerFlags, MedusaAnswer,
    Node, SpaceBuilder, Tree,
};
use rustable_codegen::handler;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

#[handler(subject_vs = "*", event = "getprocess", object_vs = "*")]
async fn getprocess_handler(ctx: &Context, args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    println!("sample process handler");

    let evtype = args.evtype;
    let mut subject = args.subject;

    subject.enter_tree(ctx, &evtype, "domains", "/").await;

    println!(
        "subject cmdline = {}",
        subject.get_attribute::<String>("cmdline")?
    );

    if subject.get_attribute::<String>("cmdline")? == "./msg_test" {
        subject.set_attribute::<u32>("med_sact", 0x0)?;
    } else {
        subject.set_attribute("med_sact", 0x3fffffff)?;
    }

    subject.update(ctx).await;

    Ok(MedusaAnswer::Allow)
}

#[handler(subject_vs = "*", event = "getipc")]
async fn getipc_handler(ctx: &Context, args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    println!("getipc");

    let mut subject = args.subject;

    subject.set_attribute("med_oact", 0x3fffffff)?;

    subject.clear_vs()?;
    subject.add_vs(*ctx.config().name_to_space_bit("all_files").unwrap())?;

    subject.update(ctx).await;

    Ok(MedusaAnswer::Allow)
}

#[handler(
    subject_vs = "all_domains",
    event = "ipc_msgsnd",
    object_vs = "all_files"
)]
async fn msgsnd_handler(_ctx: &Context, _args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    println!("ipc_msgsnd");
    Ok(MedusaAnswer::Allow)
}

#[handler(
    subject_vs = "all_domains",
    event = "ipc_msgrcv",
    object_vs = "all_files"
)]
async fn msgrcv_handler(_ctx: &Context, _args: HandlerArgs<'_>) -> Result<MedusaAnswer> {
    println!("ipc_msgrcv");
    Ok(MedusaAnswer::Allow)
}

#[rustfmt::skip]
fn create_config() -> Result<Config, ConfigError> {
    let all_files = SpaceBuilder::new()
        .with_name("all_files")
        .with_path_recursive("fs/")
        .exclude_path_recursive("fs/home/roderik")
        .exclude_space("one");

    let all_domains = SpaceBuilder::new()
        .with_name("all_domains")
        .with_path_recursive("domains/")
        .reads(["all_files", "all_domains", "home"])
        .writes(["all_files", "all_domains", "home"])
        .sees(["all_files", "all_domains", "home"]);

    let home = SpaceBuilder::new()
        .with_name("home")
        .with_path_recursive("fs/home/roderik")
        .exclude_path_recursive("fs/home/roderik/1");

    let special = SpaceBuilder::new()
        .with_name("special")
        .with_path_recursive("fs/home/roderik/1");

    let one = SpaceBuilder::new()
        .with_name("one")
        .with_path_recursive("fs/1/home");

    Config::builder()
        // the first way to define tree
        .add_tree(Tree::builder()
            .with_name("fs")
            .set_root(Node::builder()
                .with_path("/")
                .add_node(Node::builder()
                    .with_path(r"root")
                    .add_access_type(AccessType::Member, "sample")
                    .add_node(Node::builder()
                        .with_path(r"1")
                    )
                    .add_node_with_priority(1000, Node::builder()
                        .add_access_type(AccessType::Member, "sample")
                        .with_path(r".*")
                    )
                )
            )
        )

        // the second way
        .add_space(all_files)
        .add_space(all_domains)
        .add_space(home)
        .add_space(special)
        .add_space(one)

        .add_hierarchy_event_handler("getfile", "fs", Some("filename"), HandlerFlags::FROM_OBJECT)
        .add_custom_event_handler(getprocess_handler)
        .add_custom_event_handler(getipc_handler)
        .add_custom_event_handler(msgsnd_handler)
        .add_custom_event_handler(msgrcv_handler)
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
