use anyhow::Result;
use std::{fs, path::Path};

pub struct Config {
    pub db_path: String,
    pub libvirt_uri: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            db_path: String::from("/var/lib/virtus"),
            libvirt_uri: String::from("qemu:///system"),
        }
    }
}
