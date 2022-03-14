use anyhow::{Context, Result};
use rustable::force_boxed;
use rustable::medusa::{
    AuthRequestData, Config, ConfigError, Connection, EventHandler, HandlerData, MedusaAnswer,
    Node, SharedContext, Space, Tree,
};
use std::fs::OpenOptions;
use std::future::Future;
use std::pin::Pin;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

async fn getprocess_handler(
    _: &HandlerData,
    ctx: &SharedContext,
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

async fn getipc_handler(
    _: &HandlerData,
    ctx: &SharedContext,
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

async fn msgsnd_handler(
    _: &HandlerData,
    _ctx: &SharedContext,
    _auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("ipc_msgsnd");
    MedusaAnswer::Ok
}

async fn msgrcv_handler(
    _: &HandlerData,
    _ctx: &SharedContext,
    _auth_data: AuthRequestData,
) -> MedusaAnswer {
    println!("ipc_msgrcv");
    MedusaAnswer::Ok
}

#[rustfmt::skip]
fn create_config() -> Result<Config, ConfigError> {
    // TODO simplify by making a macro?
    Config::builder()
        .add_tree(Tree::builder()
            .name("fs")
            .set_root(Node::builder()
                .path("/")
                .member_of("all_files")
                .add_node(Node::builder()
                    .path(r"home")
                    .member_of("home")
                    .add_node(Node::builder()
                        .path(r"roderik")
                        .member_of("home")
                        .add_node(Node::builder()
                            .path(r"1")
                        )
                        .add_node(Node::builder()
                            .path(r".*")
                            .member_of("home")
                        )
                    )
                )
                .add_node(Node::builder()
                    .path(r"tmp")
                    .member_of("all_files")
                    .member_of("tmp")
                )
                .add_node(Node::builder()
                    .path(r".*")
                    .member_of("all_files")
                )
            )
        )
        .add_tree(Tree::builder()
            .name("domains")
            .set_root(Node::builder()
                .path("/")
                .add_node(Node::builder()
                    .path(r".*")
                    .member_of("all_domains")
                    .reads("all_domains")
                    .reads("all_files")
                    .writes("all_domains")
                    .writes("all_files")
                    .sees("all_domains")
                    .sees("all_files")
                )
            )
        )
        .add_event_handler(EventHandler::builder()
            .event("getfile")
            .with_hierarchy_handler(Some("filename"), true, "fs")
        )
        .add_event_handler(EventHandler::builder()
            .event("getprocess")
            .with_custom_handler(force_boxed!(getprocess_handler), Space::All, Some(Space::All))
        )
        .add_event_handler(EventHandler::builder()
            .event("getipc")
            .with_custom_handler(force_boxed!(getipc_handler), Space::All, None)
        )
        .add_event_handler(EventHandler::builder()
            .event("ipc_msgsnd")
            .with_custom_handler(force_boxed!(msgsnd_handler), Space::ByName("all_domains".to_owned()), Some(Space::ByName("all_files".to_owned())))
        )
        .add_event_handler(EventHandler::builder()
            .event("ipc_msgrcv")
            .with_custom_handler(force_boxed!(msgrcv_handler), Space::ByName("all_domains".to_owned()), Some(Space::ByName("all_files".to_owned())))
        )
        .build()
}

#[tokio::main]
async fn main() -> Result<()> {
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
