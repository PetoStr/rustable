use rustable::medusa::mcp::Connection;
use rustable::medusa::AuthRequestData;
use rustable::medusa::MedusaAnswer;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let write_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)?;
    let read_handle = write_handle.try_clone()?;

    let mut connection = Connection::new(write_handle, read_handle)?;

    connection.poll_loop(|_auth_data: AuthRequestData| MedusaAnswer::Ok)?;

    Ok(())
}
