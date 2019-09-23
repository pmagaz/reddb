use std::fs::{File, OpenOptions};
use std::io::Error;
use std::path::Path;
use std::result;
use std::sync::Mutex;

pub type Result<T> = result::Result<T, Error>;
use std::io::Read;

//FIXME READ
#[derive(Debug)]
pub struct Handler {
    pub file: Mutex<File>,
}

impl Handler {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let handler = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;

        Ok(Self {
            file: Mutex::new(handler),
        })
    }
}
