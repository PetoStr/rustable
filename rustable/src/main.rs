use anyhow::Result;
use rustable::medusa::{
    AuthRequestData, Config, ConfigError, Connection, Context, HandlerData, MedusaAnswer, Node,
    SpaceBuilder, Tree,
};
use rustable_codegen::handler;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

#[handler(subject = "*", event = "getprocess", object = "*")]
async fn getprocess_handler(
    _: &HandlerData,
    ctx: &Context,
    mut auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("sample process handler");

    ctx.enter_tree(&mut auth_data, "domains", "/").await;

    let subject = &mut auth_data.subject;
    println!(
        "subject cmdline = {}",
        subject.get_attribute::<String>("cmdline").unwrap()
    );

    if subject.get_attribute::<String>("cmdline").unwrap() == "./msg_test" {
        subject.set_attribute::<u32>("med_sact", 0x0).unwrap();
    } else {
        subject.set_attribute("med_sact", 0x3fffffff).unwrap();
    }

    ctx.update_object(subject).await;

    MedusaAnswer::Ok
}

#[handler(subject = "*", event = "getipc")]
async fn getipc_handler(
    _: &HandlerData,
    ctx: &Context,
    mut auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("getipc");

    auth_data
        .subject
        .set_attribute("med_oact", 0x3fffffff)
        .unwrap();

    auth_data.subject.clear_vs().unwrap();
    auth_data
        .subject
        .add_vs(*ctx.config().name_to_space_bit("all_files").unwrap())
        .unwrap();

    ctx.update_object(&auth_data.subject).await;

    MedusaAnswer::Ok
}

#[handler(subject = "all_domains", event = "ipc_msgsnd", object = "all_files")]
async fn msgsnd_handler(
    _: &HandlerData,
    _ctx: &Context,
    _auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("ipc_msgsnd");
    MedusaAnswer::Ok
}

#[handler(subject = "all_domains", event = "ipc_msgrcv", object = "all_files")]
async fn msgrcv_handler(
    _: &HandlerData,
    _ctx: &Context,
    _auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("ipc_msgrcv");
    MedusaAnswer::Ok
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
        .reads(vec!["all_files", "all_domains", "home"])
        .writes(vec!["all_files", "all_domains", "home"])
        .sees(vec!["all_files", "all_domains", "home"]);

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
                    .member_of("sample")
                    .add_node(Node::builder()
                        .with_path(r"1")
                    )
                    .add_node_with_priority(1000, Node::builder()
                        .member_of("sample")
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

        .add_hierarchy_event_handler("getfile", "fs", Some("filename"), true)
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
