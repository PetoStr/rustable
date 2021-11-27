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
        if auth_data.event == "getfile" || auth_data.event == "getprocess" {
            let mut subject = context.class_mut(&auth_data.subject).unwrap();
            println!("vs = {:?}", subject.get_attribute("vs"));

            subject.set_attribute("med_oact", vec![]);
            if auth_data.event == "getprocess" {
                subject.set_attribute("med_sact", vec![]);
            }

            context.update_object(&subject);
        }

        MedusaAnswer::Ok
    })?;

    Ok(())
}
