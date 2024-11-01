use crate::disk::Disk;
use crate::error::Error;
use serde::{Deserialize, Serialize};
use skiff::Client as SkiffClient;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Pool {
    id: Uuid,
    node_id: Uuid,
    name: Option<String>,
    path: String,
    disks: Vec<Uuid>,
}

impl Pool {
    pub async fn create(
        node_id: Uuid,
        path: &str,
        name: Option<&str>,
        client: &Arc<Mutex<SkiffClient>>,
    ) -> Result<Self, Error> {
        let pool = Self {
            id: Uuid::new_v4(),
            node_id,
            name: name.map(|s| s.to_string()),
            path: path.to_string(),
            disks: vec![],
        };

        pool.commit(client).await?;
        Ok(pool)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_node_id(&self) -> Uuid {
        self.node_id
    }

    async fn commit(&self, client: &Arc<Mutex<SkiffClient>>) -> Result<(), Error> {
        client
            .lock()
            .await
            .insert(format!("pools/{}", self.id).as_str(), self.clone())
            .await?;

        Ok(())
    }

    pub async fn get(id: Uuid, client: &Arc<Mutex<SkiffClient>>) -> Result<Option<Pool>, Error> {
        let pool = client
            .lock()
            .await
            .get::<Pool>(format!("pools/{}", id).as_str())
            .await?;

        Ok(pool)
    }

    pub async fn list(client: &Arc<Mutex<SkiffClient>>) -> Result<Vec<Pool>, Error> {
        let pool_ids = client.lock().await.list_keys("pools/").await?;

        let mut pools = Vec::new();
        for pool in pool_ids {
            pools.push(
                client
                    .lock()
                    .await
                    .get::<Pool>(pool.as_str())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }

        Ok(pools)
    }

    pub async fn create_disk(
        &mut self,
        size: usize,
        name: Option<&str>,
        client: &Arc<Mutex<SkiffClient>>,
    ) -> Result<Disk, Error> {
        let disk = Disk::create(self.id, size, name, client).await?;
        self.disks.push(disk.get_id());
        self.commit(client).await?;
        Ok(disk)
    }
}
