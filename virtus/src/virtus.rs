use crate::disk::Disk;
use crate::error::Error;
use crate::node::Node;
use crate::pool::Pool;
use skiff::{Client as SkiffClient, ElectionState, Skiff};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use virtus_client::VirtusClient;
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
    peer_clients: Arc<Mutex<HashMap<Uuid, Arc<Mutex<VirtusClient<Channel>>>>>>,
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
            peer_clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn get_cluster(&self) -> HashMap<Uuid, Node> {
        //self.skiff.get_cluster().await.unwrap()
        Node::list(&self.client)
            .await
            .unwrap()
            .into_iter()
            .map(|n| (n.get_id(), n))
            .collect()
    }

    async fn get_peers(&self) -> HashMap<Uuid, Node> {
        self.get_cluster()
            .await
            .into_iter()
            .filter(|(id, _)| id != &self.id)
            .collect()
    }

    async fn get_peer_client(
        &self,
        peer: &Uuid,
    ) -> Result<Arc<Mutex<VirtusClient<Channel>>>, Error> {
        let node = match Node::get(*peer, &self.client).await.unwrap() {
            Some(node) => node,
            None => return Err(Error::PeerNotFound),
        };

        if let Some(client) = self.peer_clients.lock().await.get(peer) {
            return Ok(client.clone());
        }

        match VirtusClient::connect(format!(
            "http://{}",
            SocketAddrV4::new(node.get_addr(), 9400)
        ))
        .await
        {
            Ok(client) => {
                let arc = Arc::new(Mutex::new(client));
                self.peer_clients
                    .lock()
                    .await
                    .insert(peer.to_owned(), arc.clone());
                Ok(arc)
            }
            Err(_) => Err(Error::PeerConnectFailed),
        }
    }

    async fn drop_client(&self, id: Uuid) {
        self.peer_clients.lock().await.remove(&id);
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
        let id = match Uuid::from_str(&request.into_inner().id) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid node ID")),
        };

        match Node::get(id, &self.client).await {
            Ok(node) => Ok(Response::new(GetNodeReply {
                node: node.map(|n| n.into()),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
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
        let (mut metadata, extensions, inner) = request.into_parts();

        let node_id = match Uuid::parse_str(&inner.node) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid node ID")),
        };

        if Node::get(node_id, &self.client).await.unwrap().is_none() {
            return Err(Status::invalid_argument("Node not found"));
        }

        match self.skiff.get_election_state().await {
            ElectionState::Leader => {
                if inner.node != self.id.to_string() {
                    let client = self.get_peer_client(&node_id).await;
                    if let Ok(client_inner) = client {
                        // Indicate that this is forwarded from leader
                        // Todo: more rigorous way to indicate forwarded request
                        metadata.append("forwarded", MetadataValue::from_static(""));
                        return client_inner
                            .lock()
                            .await
                            .add_pool(Request::from_parts(metadata, extensions, inner))
                            .await;
                    }

                    return Err(Status::internal("failed to connect to node"));
                }
            }
            ElectionState::Follower(leader) => {
                // Check if the request is from the leader
                if metadata.get("forwarded").is_none() {
                    // Forward to leader
                    let client = self.get_peer_client(&leader).await;
                    if let Ok(client_inner) = client {
                        return client_inner
                            .lock()
                            .await
                            .add_pool(Request::from_parts(metadata, extensions, inner))
                            .await;
                    }

                    return Err(Status::internal("failed to forward request to leader"));
                }
            }
            ElectionState::Candidate => return Err(Status::internal("no skiff leader elected")),
        }

        match Pool::create(
            self.id,
            inner.path.as_str(),
            inner.name.as_deref(),
            &self.client,
        )
        .await
        {
            Ok(pool) => {
                return Ok(Response::new(AddPoolReply {
                    success: true,
                    id: Some(pool.get_id().to_string()),
                }))
            }
            Err(e) => return Err(Status::internal(e.to_string())),
        }
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
        let id = match Uuid::from_str(&request.into_inner().id) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid pool ID")),
        };

        match Pool::get(id, &self.client).await {
            Ok(pool) => Ok(Response::new(GetPoolReply {
                pool: pool.map(|p| p.into()),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
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
        let (mut metadata, extensions, inner) = request.into_parts();

        let pool_id = match Uuid::parse_str(&inner.pool) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid pool ID")),
        };

        let pool = match Pool::get(pool_id, &self.client).await.unwrap() {
            Some(pool) => pool,
            None => return Err(Status::invalid_argument("Pool not found")),
        };

        let node_id = pool.get_node_id();

        match self.skiff.get_election_state().await {
            ElectionState::Leader => {
                if node_id != self.id {
                    let client = self.get_peer_client(&node_id).await;
                    if let Ok(client_inner) = client {
                        // Indicate that this is forwarded from leader
                        // Todo: more rigorous way to indicate forwarded request
                        metadata.append("forwarded", MetadataValue::from_static(""));
                        return client_inner
                            .lock()
                            .await
                            .add_disk(Request::from_parts(metadata, extensions, inner))
                            .await;
                    }

                    return Err(Status::internal("failed to connect to node"));
                }
            }
            ElectionState::Follower(leader) => {
                // Check if the request is from the leader
                if metadata.get("forwarded").is_none() {
                    // Forward to leader
                    let client = self.get_peer_client(&leader).await;
                    if let Ok(client_inner) = client {
                        return client_inner
                            .lock()
                            .await
                            .add_disk(Request::from_parts(metadata, extensions, inner))
                            .await;
                    }

                    return Err(Status::internal("failed to forward request to leader"));
                }
            }
            ElectionState::Candidate => return Err(Status::internal("no skiff leader elected")),
        }

        match Disk::create(
            pool_id,
            inner.size_gb as usize,
            inner.name.as_deref(),
            &self.client,
        )
        .await
        {
            Ok(disk) => {
                return Ok(Response::new(AddDiskReply {
                    success: true,
                    id: Some(disk.get_id().to_string()),
                }))
            }
            Err(e) => return Err(Status::internal(e.to_string())),
        }
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
        let id = match Uuid::from_str(&request.into_inner().id) {
            Ok(id) => id,
            Err(_) => return Err(Status::invalid_argument("Invalid disk ID")),
        };

        match Disk::get(id, &self.client).await {
            Ok(disk) => Ok(Response::new(GetDiskReply {
                disk: disk.map(|d| d.into()),
            })),
            Err(e) => return Err(Status::internal(e.to_string())),
        }
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

    async fn add_network(
        &self,
        request: Request<AddNetworkRequest>,
    ) -> Result<Response<AddNetworkReply>, Status> {
        todo!()
    }

    async fn remove_network(
        &self,
        request: Request<RemoveNetworkRequest>,
    ) -> Result<Response<RemoveNetworkReply>, Status> {
        todo!()
    }

    async fn get_network(
        &self,
        request: Request<GetNetworkRequest>,
    ) -> Result<Response<GetNetworkReply>, Status> {
        todo!()
    }

    async fn list_networks(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ListNetworksReply>, Status> {
        todo!()
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
        if Path::exists(Path::new("target/tmp/test")) {
            fs::remove_dir_all("target/tmp/test")?;
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
            .join_cluster(vec!["127.0.0.1".parse().unwrap()])
            .build()
            .unwrap())
    }

    async fn get_client(address: &str) -> Result<VirtusClient<Channel>, anyhow::Error> {
        Ok(VirtusClient::connect(format!(
            "http://{}",
            SocketAddrV4::new(address.parse().unwrap(), 9400)
        ))
        .await?)
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
    async fn two_node_cluster() {
        let leader = get_virtus().unwrap();
        let leader_clone = leader.clone();
        let handle = tokio::spawn(async move {
            let _ = leader_clone.start().await;
        });

        // Give leader time to elect itself
        // Todo: again, need more reliable method for determining when servers are ready
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let follower = get_follower("127.0.0.2").unwrap();
        let follower_clone = follower.clone();
        let _ = tokio::spawn(async move {
            let _ = follower_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let leader_cluster = leader.skiff.get_cluster().await.unwrap();
        let follower_cluster = follower.skiff.get_cluster().await.unwrap();

        assert_eq!(leader_cluster, follower_cluster);
        assert_eq!(2, leader_cluster.len());
        assert_eq!(2, Node::list(&leader.client).await.unwrap().len());
    }

    #[tokio::test]
    #[serial]
    async fn add_pool_two_nodes() {
        let leader = get_virtus().unwrap();
        let leader_clone = leader.clone();
        let handle = tokio::spawn(async move {
            let _ = leader_clone.start().await;
        });

        // Give leader time to elect itself
        // Todo: again, need more reliable method for determining when servers are ready
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let follower = get_follower("127.0.0.2").unwrap();
        let follower_clone = follower.clone();
        let _ = tokio::spawn(async move {
            let _ = follower_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let mut follower_client = get_client("127.0.0.2").await.unwrap();
        let pool = follower_client
            .add_pool(Request::new(AddPoolRequest {
                name: Some("test".to_string()),
                path: "target/tmp/test/follower_pool".to_string(),
                node: follower.id.to_string(),
            }))
            .await;

        assert_eq!(1, Pool::list(&leader.client).await.unwrap().len());
        assert!(Path::exists(Path::new("target/tmp/test/follower_pool")))
    }

    #[tokio::test]
    #[serial]
    async fn add_disk_two_nodes() {
        let leader = get_virtus().unwrap();
        let leader_clone = leader.clone();
        let handle = tokio::spawn(async move {
            let _ = leader_clone.start().await;
        });

        // Give leader time to elect itself
        // Todo: again, need more reliable method for determining when servers are ready
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let follower = get_follower("127.0.0.2").unwrap();
        let follower_clone = follower.clone();
        let _ = tokio::spawn(async move {
            let _ = follower_clone.start().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let mut follower_client = get_client("127.0.0.2").await.unwrap();
        let pool = follower_client
            .add_pool(Request::new(AddPoolRequest {
                name: Some("test".to_string()),
                path: "target/tmp/test/follower_pool".to_string(),
                node: follower.id.to_string(),
            }))
            .await
            .unwrap()
            .into_inner()
            .id
            .unwrap();

        let disk = follower_client
            .add_disk(Request::new(AddDiskRequest {
                name: Some("test_disk".into()),
                pool,
                size_gb: 1,
            }))
            .await
            .unwrap()
            .into_inner()
            .id
            .unwrap();

        assert_eq!(1, Disk::list(&leader.client).await.unwrap().len());
        let filename = format!("target/tmp/test/follower_pool/{}.qcow2", disk);
        assert!(Path::exists(Path::new(&filename)));
    }
}
