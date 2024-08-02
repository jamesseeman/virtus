pub mod ovs;
pub mod error;
pub mod domain;
pub mod vm;
pub mod config;

use std::{fs, path::Path};

use error::VirtusError;

pub fn connect(conf: config::Config) -> Result<Connection, VirtusError> {
    if !Path::new(&conf.db_path).exists() {
        fs ::create_dir(&conf.db_path)?;
    }

    Ok(Connection{
        virt: virt::connect::Connect::open(&conf.libvirt_uri)?,
        db: sled::open(format!("{}/config", &conf.db_path))?,
    })
}

pub struct Connection {
    pub virt: virt::connect::Connect,
    pub db: sled::Db,
}

impl Connection {
    pub fn close(&mut self) -> Result<(), VirtusError> {
        self.virt.close()?;
        Ok(())
    }
}
