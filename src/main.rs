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
            .evtype(&auth_data.evtype_id)
            .expect("Unknown event")
            .name();

        let to_fetch = context.class(&auth_data.subject_id).unwrap().clone();
        println!("sample fetch: {:?}", context.fetch_object(to_fetch));

        if evtype == "getfile" || evtype == "getprocess" {
            let mut subject = context.class_mut(&auth_data.subject_id).unwrap();
            // Critical section, do not fetch objects, because write lock is being used.
            // TODO This is an issue, becase objects are currently tied to classes in 1:1 relation.
            println!("vs = {:?}", subject.get_attribute("vs"));

            subject.set_attribute("med_oact", vec![]);
            if evtype == "getprocess" {
                subject.set_attribute("med_sact", vec![]);
            }

            context.update_object(&subject);
        } // Critical section ends here.

        MedusaAnswer::Ok
    })?;

    Ok(())
}
