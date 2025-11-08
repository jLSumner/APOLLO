// src/core/security.rs

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, Clone, Default)]
pub struct SecurityCodes {
    pub codes: HashMap<String, String>,
}

impl SecurityCodes {
    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut codes = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            if let Some((key, value)) = line.split_once(">>") {
                let formatted_key = key.trim().replace(" ", "");
                codes.insert(formatted_key, value.trim().to_string());
            }
        }

        Ok(Self { codes })
    }

    pub fn get_code(&self, action: &str) -> Option<&String> {
        self.codes.get(action)
    }
}