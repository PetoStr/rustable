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
        let evtype = auth_data.evtype;
        let evtype_name = evtype.name();
        let mut subject = auth_data.subject;

        println!("sample fetch: {:?}", context.fetch_object(&subject));

        if evtype_name == "getfile" || evtype_name == "getprocess" {
            println!("vs = {:?}", subject.get_attribute("vs"));
            if evtype_name == "getfile" {
                let filename = rustable::cstr_to_string(evtype.get_attribute("filename"));
                println!("filename: `{}`\n", filename);
            }

            subject.set_attribute("med_oact", vec![]);
            if evtype_name == "getprocess" {
                subject.set_attribute("med_sact", vec![]);
            }

            let update_answer = context.update_object(&subject);
            println!("update_answer = {:?}", update_answer);
        }

        MedusaAnswer::Ok
    })?;

    Ok(())
}
