use rustable::medusa::mcp::{Connection, SharedContext};
use rustable::medusa::{AuthRequestData, MedusaAnswer};
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
            let mut classes = context.classes.lock().unwrap();
            let subject = classes.get_mut(&auth_data.subject).unwrap();
            println!("vs = {:?}", subject.get_attribute("vs"));

            subject.set_attribute("med_oact", vec![]);
            if auth_data.event == "getprocess" {
                subject.set_attribute("med_sact", vec![]);
            }

            let packed_attrs = subject.pack_attributes();
            context.update_object(auth_data.subject, &packed_attrs);
        }

        MedusaAnswer::Ok
    })?;

    Ok(())
}
