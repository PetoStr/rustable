use anyhow::{Context, Result};
use async_trait::async_trait;
use rustable::cstr_to_string;
use rustable::medusa::{
    AuthRequestData, Config, Connection, EventHandler, MedusaAnswer, SharedContext, Tree, TreeError,
};
use tokio::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

struct SampleFsHandler;

#[async_trait]
impl EventHandler for SampleFsHandler {
    async fn handle(&self, ctx: &SharedContext, auth_data: AuthRequestData) -> MedusaAnswer {
        println!("sample fs handler");

        let mut subject = auth_data.subject;
        subject.clear_object_act().unwrap();

        let update_answer = ctx.update_object(&subject).await;
        println!("update_answer = {:?}\n", update_answer);

        MedusaAnswer::Ok
    }
}

struct SampleProcessHandler;

#[async_trait]
impl EventHandler for SampleProcessHandler {
    async fn handle(&self, ctx: &SharedContext, auth_data: AuthRequestData) -> MedusaAnswer {
        println!("sample process handler");

        let mut subject = auth_data.subject;
        println!(
            "subject cmdline = {}",
            cstr_to_string(subject.get_attribute("cmdline").unwrap())
        );

        subject.clear_object_act().unwrap();
        subject.clear_subject_act().unwrap();

        let update_answer = ctx.update_object(&subject).await;
        println!("update_answer = {:?}\n", update_answer);

        MedusaAnswer::Ok
    }
}

#[rustfmt::skip]
fn create_config() -> Result<Config, TreeError> {
    // TODO simplify by making a macro?
    let fs = Tree::builder_with_attribute("getfile", "filename")
        .begin_node("name0", "/")
            .with_handler(SampleFsHandler)

            .begin_node("name1", "usr")
                .begin_node("name2", "bin")
                    .begin_node("name2", r".*")
                        .with_handler(SampleFsHandler)
                    .end_node()?
                .end_node()?
            .end_node()?

            .begin_node("name3", "share")
            .end_node()?

            .begin_node("name4", "bin")
            .end_node()?

        .end_node()?
        .build()?;

    let domain = Tree::builder("getprocess")
        .begin_node("name5", r".*")
            .with_handler(SampleProcessHandler)
        .end_node()?
        .build()?;

    Ok(Config::builder()
        .add_tree(fs)
        .add_tree(domain)
        .build())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = create_config().context("Failed to create config")?;
    println!("{:?}", config);

    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)
        .await?;
    let read_handle = write_handle.try_clone().await?;

    let mut connection = Connection::new(write_handle, read_handle, config)
        .await
        .context("Connection failed")?;
    connection.run().await.context("Communication failed")?;

    Ok(())
}
