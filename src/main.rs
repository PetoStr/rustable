use rustable::medusa::mcp::Connection;
use rustable::medusa::{AuthRequestData, MedusaAnswer, SharedContext};
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)?;
    let read_handle = write_handle.try_clone()?;

    let mut connection = Connection::new(write_handle, read_handle)?;

    connection.poll_loop(|context: &SharedContext, auth_data: AuthRequestData| {
        let evtype = context
            .empty_evtype(&auth_data.evtype_id)
            .expect("Unknown event")
            .name();

        let mut subject = auth_data.subject;

        println!("sample fetch: {:?}", context.fetch_object(&subject));

        if evtype == "getfile" || evtype == "getprocess" {
            println!("vs = {:?}", subject.get_attribute("vs"));

            subject.set_attribute("med_oact", vec![]);
            if evtype == "getprocess" {
                subject.set_attribute("med_sact", vec![]);
            }

            context.update_object(&subject);
        }

        MedusaAnswer::Ok
    })?;

    Ok(())
}
