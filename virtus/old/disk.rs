use std::{fs, path::Path, process::Command};

use crate::{Connection, Error};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Disk {
    id: Uuid,
    filename: String,
    size: u64,
}

impl Disk {
    pub fn new(size: u64, conn: &Connection) -> Result<Self> {
        let disk_dir = format!("{}/disks", &conn.data_dir);
        if !Path::exists(Path::new(&disk_dir)) {
            fs::create_dir(&disk_dir)?;
        }

        let id = Uuid::new_v4();
        let filename = format!("{}/{}.qcow2", disk_dir, id);

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("qemu-img create -f qcow2 {} {}", filename, size))
            .output()?;

        if output.status.success() {
            let disk = Self { id, filename, size };
            conn.db
                .open_tree("disks")?
                .insert(id, bincode::serialize(&disk)?)?;
            Ok(disk)
        } else {
            Err(Error::DiskError.into())
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    pub fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        // todo: check if a VM is using disk
        // todo: handle when file has already been deleted
        let tree = conn.db.open_tree("disks")?;
        if let Some(found) = tree.get(id)? {
            let disk: Disk = bincode::deserialize(&found)?;
            std::fs::remove_file(disk.filename)?;
        }

        tree.remove(id)?;
        Ok(())
    }

    pub fn delete(self, conn: &Connection) -> Result<String> {
        // todo: check if a VM is using disk
        // todo: handle when file has already been deleted
        std::fs::remove_file(&self.filename)?;
        conn.db.open_tree("disks")?.remove(self.id)?;
        Ok(self.filename.clone())
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("disks")?.get(id)? {
            Some(disk) => Ok(Some(bincode::deserialize(&disk)?)),
            None => Ok(None),
        }
    }

    pub fn list(conn: &Connection) -> Result<Vec<Uuid>> {
        let disks: Vec<Uuid> = conn
            .db
            .open_tree("disks")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|option| Uuid::from_slice(&option.0).ok())
            .collect();

        Ok(disks)
    }
}
