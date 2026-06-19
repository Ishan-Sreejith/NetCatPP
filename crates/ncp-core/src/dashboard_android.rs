use std::error::Error;

pub async fn run() -> Result<(), Box<dyn Error>> {
    eprintln!("Dashboard is not supported on Android yet.");
    Ok(())
}
