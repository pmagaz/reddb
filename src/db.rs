use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(Debug)]
pub struct Db {
    file_name: String,
}

impl Db {
    pub fn new(file_name: String) -> Db {
        Db {
            file_name: file_name,
        }
    }

    pub fn start(self) -> Result<Db, std::io::Error> {
        if self.file_exits(&self.file_name) {
            Ok(self)
        } else {
            let file = self.create_file(&self.file_name)?;
            Ok(self)
        }
    }

    pub fn file_exits(&self, file_name: &String) -> bool {
        Path::new(&file_name).exists()
    }

    pub fn create_file(&self, file_name: &String) -> std::io::Result<File> {
        let file = File::create(&file_name)?;
        Ok(file)
    }
}
