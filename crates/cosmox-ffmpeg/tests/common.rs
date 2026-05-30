use std::{
    error::Error,
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

pub fn media(url: &'static str) -> Result<PathBuf, Box<dyn Error>> {
    let mut target_path = PathBuf::from("/tmp");
    let filename = url.split('/').next_back().unwrap();
    target_path.push(filename);

    if target_path.exists() {
        return Ok(target_path);
    }

    let mut response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        return Err(format!("Network error, HTTP status code: {}", response.status()).into());
    }

    let mut dest_file = File::create(&target_path)?;

    io::copy(&mut response, &mut dest_file)?;

    dest_file.flush()?;
    Ok(target_path)
}
