use std::fs::File;
use std::io::{BufRead, BufReader, Error, Write};
use std::path::Path;

#[derive(Debug)]
pub struct Db {
    pub handler: FileHandler,
}

#[derive(Debug)]
pub struct FileHandler {
    file_name: String,
    handler: File,
}

impl FileHandler {
    pub fn new(file_name: String) -> FileHandler {
        let handler;
        let exists = Path::new(&file_name).exists();
        if exists {
            handler = File::open(&file_name).unwrap();
        } else {
            handler = File::create(&file_name).unwrap();
        }
        FileHandler {
            file_name: file_name,
            handler: handler,
        }
    }
}

impl Db {
    pub fn new(file_name: String) -> Db {
        Db {
            handler: FileHandler::new(file_name),
        }
    }
    /*pub fn start(self) -> Result<Db, std::io::Error> {
        if self.file_exits(&self.file_name) {
            Ok(self)
        } else {
            let handler = self.create_file(&self.file_name)?;
            self.handler = handler;
            Ok(self)
        }
    }*/
}
