use rustable::medusa::mcp::Connection;
use rustable::medusa::AuthRequestData;
use rustable::medusa::MedusaAnswer;
use std::fs::OpenOptions;

const MEDUSA_FILE_NAME: &str = "/dev/medusa";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MEDUSA_FILE_NAME)?;
    let mut connection = Connection::new(file)?;

    connection.poll_loop(|_auth_data: AuthRequestData| MedusaAnswer::Ok)?;

    Ok(())
}
