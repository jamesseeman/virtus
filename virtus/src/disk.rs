use crate::pool::Pool;
use crate::{error::Error, virtus::virtus_proto};
use serde::{Deserialize, Serialize};
use skiff::Client as SkiffClient;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Disk {
    id: Uuid,
    pool_id: Uuid,
    name: Option<String>,
    size: usize,
    // source: Option<Image>,
    // snapshots: Vec<Snapshot>,
}

impl Disk {
    pub async fn create(
        pool_id: Uuid,
        size: usize,
        name: Option<&str>,
        client: &Arc<Mutex<SkiffClient>>,
    ) -> Result<Self, Error> {
        let pool = match Pool::get(pool_id, client).await? {
            Some(pool) => pool,
            None => return Err(Error::PoolNotFound),
        };

        let disk_id = Uuid::new_v4();

        let filename = Path::join(
            Path::new(&pool.get_path()),
            Path::new(&format!("{}.qcow2", disk_id)),
        );

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "qemu-img create -f qcow2 {} {}",
                filename.to_str().unwrap(),
                size
            ))
            .output()?;

        match output.status.success() {
            true => {
                let disk = Self {
                    id: disk_id,
                    pool_id,
                    name: name.map(|s| s.to_string()),
                    size,
                };

                disk.commit(client).await?;
                Ok(disk)
            }
            false => Err(Error::CommandFailed(
                String::from_utf8(output.stderr).unwrap(),
            )),
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_pool_id(&self) -> Uuid {
        self.pool_id
    }

    async fn commit(&self, client: &Arc<Mutex<SkiffClient>>) -> Result<(), Error> {
        client
            .lock()
            .await
            .insert(format!("disks/{}", self.id).as_str(), self.clone())
            .await?;

        Ok(())
    }

    pub async fn get(id: Uuid, client: &Arc<Mutex<SkiffClient>>) -> Result<Option<Disk>, Error> {
        let disk = client
            .lock()
            .await
            .get::<Disk>(format!("disks/{}", id).as_str())
            .await?;

        Ok(disk)
    }

    pub async fn list(client: &Arc<Mutex<SkiffClient>>) -> Result<Vec<Disk>, Error> {
        let disk_ids = client.lock().await.list_keys("disks/").await?;

        let mut disks = Vec::new();
        for disk in disk_ids {
            disks.push(
                client
                    .lock()
                    .await
                    .get::<Disk>(disk.as_str())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }

        Ok(disks)
    }
}

impl From<Disk> for virtus_proto::Disk {
    fn from(val: Disk) -> Self {
        virtus_proto::Disk {
            id: val.id.to_string(),
            pool: val.pool_id.to_string(),
            name: val.name,
            size: val.size as u64,
        }
    }
}
