use rustable::medusa::mcp::Connection;
use rustable::medusa::{AuthRequestData, MedusaAnswer, SharedContext};
use tokio::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

async fn process(context: SharedContext, auth_data: AuthRequestData) -> MedusaAnswer {
    let evtype = auth_data.evtype;
    let evtype_name = evtype.name();
    let mut subject = auth_data.subject;

    println!("sample fetch: {:?}", context.fetch_object(&subject).await);

    if evtype_name == "getfile" || evtype_name == "getprocess" {
        println!("vs = {:?}", subject.get_attribute("vs"));
        if evtype_name == "getfile" {
            let filename = rustable::cstr_to_string(evtype.get_attribute("filename"));
            println!("filename: `{}`\n", filename);
        }

        subject.clear_object_act();
        if evtype_name == "getprocess" {
            subject.clear_subject_act();
        }

        let update_answer = context.update_object(&subject).await;
        println!("update_answer = {:?}", update_answer);
    }

    MedusaAnswer::Ok
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)
        .await?;
    let read_handle = write_handle.try_clone().await?;

    let mut connection = Connection::new(write_handle, read_handle).await?;

    connection.poll_loop(process).await?;

    Ok(())
}
