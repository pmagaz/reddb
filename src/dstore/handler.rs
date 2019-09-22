use std::sync::{Mutex};
use std::fs::{File, OpenOptions};
use std::io::Error;
use std::io::prelude::*;
use std::io::{self, prelude::*, BufReader};

use std::path::Path;
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Handler {
    pub file: Mutex<File>,
}

impl Handler {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let handler = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        Ok(Self { file: Mutex::new(handler) })
    }
}
