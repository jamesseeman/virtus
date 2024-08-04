mod config;
mod disk;
mod error;
mod image;
mod interface;
mod network;
pub mod ovs;
mod vm;

pub use config::Config;
pub use disk::Disk;
pub use error::Error;
pub use image::Image;
pub use interface::Interface;
pub use network::Network;
pub use vm::*;

use std::{fs, path::Path};

pub const KILOBYTE: u64 = 1024;
pub const MEGABYTE: u64 = 1024 * 1024;
pub const GIGABYTE: u64 = 1024 * 1024 * 1024;
pub const TERABYTE: u64 = 1024 * 1024 * 1024 * 1024;

/// Returns a Connection struct containing connections to the libvirt socket and sled KV database.
///
/// # Examples
/// ```
/// let conf = virtus::config::Config::new();
/// let mut conn = virtus::connect(&conf)?;
///
/// conn.close()?;
/// ```
pub fn connect(conf: &config::Config) -> Result<Connection, Error> {
    if !Path::new(&conf.data_dir).exists() {
        fs::create_dir(&conf.data_dir)?;
    }

    Ok(Connection {
        virt: virt::connect::Connect::open(&conf.libvirt_uri)?,
        db: sled::open(format!("{}/config", &conf.data_dir))?,
        data_dir: conf.data_dir.clone(),
    })
}

pub struct Connection {
    pub virt: virt::connect::Connect,
    pub db: sled::Db,
    pub data_dir: String,
}

impl Connection {
    /// Closes the connection to the hypervisor.
    pub fn close(&mut self) -> Result<(), Error> {
        self.virt.close()?;
        Ok(())
    }
}
