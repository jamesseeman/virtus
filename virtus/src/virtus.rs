use crate::disk::Disk;
use crate::error::Error;
use crate::node::Node;
use crate::pool::Pool;
use skiff::{Client as SkiffClient, Skiff};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use virtus_proto::virtus_server::VirtusServer;
use virtus_proto::*;

pub mod virtus_proto {
    tonic::include_proto!("virtus");
}

#[derive(Clone)]
pub struct Virtus {
    id: Uuid,
    address: Ipv4Addr,
    skiff: Arc<Skiff>,
    client: Arc<Mutex<SkiffClient>>,
}

impl Virtus {
    // Todo: add token gen for authenticated join to cluster
    pub fn new(
        id: Uuid,
        address: Ipv4Addr,
        data_dir: String,
        peers: Vec<Ipv4Addr>,
    ) -> Result<Self, Error> {
        Ok(Self {
            id,
            address,
            skiff: Arc::new(Skiff::new(id, address, data_dir, peers.clone())?),
            client: Arc::new(Mutex::new(SkiffClient::new(vec![address]))),
        })
    }

    pub async fn start(self) -> Result<(), anyhow::Error> {
        let skiff_service = self.skiff.initialize_service();
        let virtus = self.clone();
        let addr = self.address;
        let _handle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
            Server::builder()
                .add_service(skiff_service)
                .add_service(VirtusServer::new(virtus))
                .serve(SocketAddr::new(addr.into(), 9400))
                .await?;

            Ok(())
        });

        while !self.skiff.is_leader_elected().await {
            // todo: ideally skiff implements a better way to notify on ready without polling
            // Wait for one election timeout
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        let hostname = hostname::get().unwrap().into_string().unwrap();

        // Create a node associated with this server
        // The node's id matches virtus id
        Node::create(self.id, hostname.as_str(), self.address, &self.client).await?;

        Ok(())
    }
}

#[tonic::async_trait]
impl virtus_proto::virtus_server::Virtus for Virtus {
    async fn add_node(
        &self,
        request: Request<AddNodeRequest>,
    ) -> Result<Response<AddNodeReply>, Status> {
        todo!()
    }

    async fn remove_node(
        &self,
        request: Request<RemoveNodeRequest>,
    ) -> Result<Response<RemoveNodeReply>, Status> {
        todo!()
    }

    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeReply>, Status> {
        todo!()
    }

    async fn list_nodes(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ListNodesReply>, Status> {
        match Node::list(&self.client).await {
            Ok(nodes) => Ok(Response::new(ListNodesReply {
                nodes: nodes.into_iter().map(|n| n.get_id().to_string()).collect(),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
    }

    async fn add_pool(
        &self,
        request: Request<AddPoolRequest>,
    ) -> Result<Response<AddPoolReply>, Status> {
        todo!()
    }

    async fn remove_pool(
        &self,
        request: Request<RemovePoolRequest>,
    ) -> Result<Response<RemovePoolReply>, Status> {
        todo!()
    }

    async fn get_pool(
        &self,
        request: Request<GetPoolRequest>,
    ) -> Result<Response<GetPoolReply>, Status> {
        todo!()
    }

    async fn list_pools(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ListPoolsReply>, Status> {
        match Pool::list(&self.client).await {
            Ok(pools) => Ok(Response::new(ListPoolsReply {
                pools: pools.into_iter().map(|p| p.get_id().to_string()).collect(),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
    }

    async fn add_disk(
        &self,
        request: Request<AddDiskRequest>,
    ) -> Result<Response<AddDiskReply>, Status> {
        todo!()
    }

    async fn remove_disk(
        &self,
        request: Request<RemoveDiskRequest>,
    ) -> Result<Response<RemoveDiskReply>, Status> {
        todo!()
    }

    async fn get_disk(
        &self,
        request: Request<GetDiskRequest>,
    ) -> Result<Response<GetDiskReply>, Status> {
        todo!()
    }

    async fn list_disks(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ListDisksReply>, Status> {
        match Disk::list(&self.client).await {
            Ok(disks) => Ok(Response::new(ListDisksReply {
                disks: disks.into_iter().map(|d| d.get_id().to_string()).collect(),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::pool::Pool;
    use crate::Builder;
    use serial_test::serial;

    fn get_virtus() -> Result<Virtus, anyhow::Error> {
        let dir = String::from("target/tmp/test/127.0.0.1");
        if Path::exists(Path::new(&dir)) {
            fs::remove_dir_all(&dir)?;
        }

        Ok(Builder::new()
            .bind("127.0.0.1".parse().unwrap())
            .set_dir(&dir)
            .build()
            .unwrap())
    }

    fn get_follower(address: &str) -> Result<Virtus, anyhow::Error> {
        let dir = format!("target/tmp/test/{}", &address);
        if Path::exists(Path::new(&dir)) {
            fs::remove_dir_all(&dir)?;
        }

        Ok(Builder::new()
            .bind(address.parse().unwrap())
            .set_dir(&dir)
            .build()
            .unwrap())
    }

    #[tokio::test]
    #[serial]
    async fn start_server() {
        let virtus = get_virtus().unwrap();

        let virtus_clone = virtus.clone();
        let handle = tokio::spawn(async move {
            let _ = virtus_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        assert_eq!(
            1,
            virtus
                .client
                .lock()
                .await
                .list_keys("nodes/")
                .await
                .unwrap()
                .len()
        );
    }

    #[tokio::test]
    #[serial]
    async fn list_nodes() {
        let virtus = get_virtus().unwrap();

        let virtus_clone = virtus.clone();
        let handle = tokio::spawn(async move {
            let _ = virtus_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        assert_eq!(1, Node::list(&virtus.client).await.unwrap().len());
    }

    #[tokio::test]
    #[serial]
    async fn create_pool() {
        let virtus = get_virtus().unwrap();

        let virtus_clone = virtus.clone();
        let handle = tokio::spawn(async move {
            let _ = virtus_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        assert_eq!(
            Vec::<Pool>::new(),
            Pool::list(&virtus.client).await.unwrap()
        );

        let mut node = Node::list(&virtus.client)
            .await
            .unwrap()
            .first()
            .unwrap()
            .clone();

        let pool = node
            .create_pool("target/tmp/test/pool1", None, &virtus.client)
            .await
            .unwrap();

        assert_eq!(1, Pool::list(&virtus.client).await.unwrap().len());

        let node = Node::list(&virtus.client)
            .await
            .unwrap()
            .first()
            .unwrap()
            .clone();

        assert_eq!(pool.get_node_id(), node.get_id());

        let node_pools: Vec<Uuid> = node
            .list_pools(virtus.client.clone())
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.get_id())
            .collect();

        assert_eq!(vec![pool.get_id()], node_pools);
    }

    #[tokio::test]
    #[serial]
    async fn two_node_cluster() {}
}
