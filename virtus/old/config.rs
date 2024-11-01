use anyhow::Result;
use std::{fs, path::Path};

pub struct Config {
    pub data_dir: String,
    pub libvirt_uri: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            data_dir: String::from("/var/lib/virtus"),
            libvirt_uri: String::from("qemu:///system"),
        }
    }
}
