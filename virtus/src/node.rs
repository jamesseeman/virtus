use crate::error::Error;
use crate::pool::Pool;
use serde::{Deserialize, Serialize};
use skiff::Client as SkiffClient;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Node {
    id: Uuid,
    address: Ipv4Addr,
    hostname: String,
    pools: Vec<Uuid>,
}

impl Node {
    pub async fn create(
        id: Uuid,
        hostname: &str,
        address: Ipv4Addr,
        client: &Arc<Mutex<SkiffClient>>,
    ) -> Result<Self, Error> {
        let node = Self {
            id,
            hostname: String::from(hostname),
            address,
            pools: vec![],
        };

        node.commit(client).await?;
        Ok(node)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    async fn commit(&self, client: &Arc<Mutex<SkiffClient>>) -> Result<(), Error> {
        client
            .lock()
            .await
            .insert(format!("nodes/{}", self.id).as_str(), self.clone())
            .await?;

        Ok(())
    }

    pub async fn get(id: Uuid, client: &Arc<Mutex<SkiffClient>>) -> Result<Option<Node>, Error> {
        let node = client
            .lock()
            .await
            .get::<Node>(format!("nodes/{}", id).as_str())
            .await?;

        Ok(node)
    }

    pub async fn list(client: &Arc<Mutex<SkiffClient>>) -> Result<Vec<Node>, Error> {
        let node_ids = client.lock().await.list_keys("nodes/").await?;

        let mut nodes = Vec::new();
        for node in node_ids {
            nodes.push(
                client
                    .lock()
                    .await
                    .get::<Node>(node.as_str())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }

        Ok(nodes)
    }

    pub async fn create_pool(
        &mut self,
        path: &str,
        name: Option<&str>,
        client: &Arc<Mutex<SkiffClient>>,
    ) -> Result<Pool, Error> {
        let pool = Pool::create(self.id, path, name, client).await?;
        self.pools.push(pool.get_id());
        self.commit(client).await?;
        Ok(pool)
    }

    pub async fn list_pools(&self, client: Arc<Mutex<SkiffClient>>) -> Result<Vec<Pool>, Error> {
        let mut pools = Vec::<Pool>::new();
        for pool in &self.pools {
            pools.push(Pool::get(*pool, &client).await.unwrap().unwrap());
        }

        Ok(pools)
    }
}
