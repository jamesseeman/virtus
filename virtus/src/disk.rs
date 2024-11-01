use crate::error::Error;
use serde::{Deserialize, Serialize};
use skiff::Client as SkiffClient;
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
        let disk = Self {
            id: Uuid::new_v4(),
            pool_id,
            name: name.map(|s| s.to_string()),
            size,
        };

        disk.commit(client).await?;
        Ok(disk)
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
