use anyhow::{Context, Result};
use async_trait::async_trait;
use rustable::cstr_to_string;
use rustable::medusa::{
    AuthRequestData, Config, ConfigError, Connection, EventHandler, Handler, HandlerData,
    MedusaAnswer, Node, SharedContext, Space, Tree,
};
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

struct SampleGetProcessHandler;

#[async_trait]
impl Handler for SampleGetProcessHandler {
    async fn handle(
        &self,
        _: &HandlerData,
        ctx: &SharedContext,
        mut auth_data: AuthRequestData,
    ) -> MedusaAnswer {
        println!("sample process handler");

        let subject = &auth_data.subject;
        println!(
            "subject cmdline = {}",
            cstr_to_string(subject.get_attribute("cmdline").unwrap())
        );

        ctx.enter_tree(&mut auth_data, "domains", "/").await;

        MedusaAnswer::Ok
    }
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
            .with_custom_handler(SampleGetProcessHandler, Space::All, None)
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
