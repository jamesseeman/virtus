use crate::{error::Error, virtus::Virtus};
use std::net::Ipv4Addr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Builder {
    id: Uuid,
    bind_address: Ipv4Addr,
    data_dir: String,

    // If empty, we are the leader of a new cluster
    // Otherwise, we are a follower in an existing cluster
    peers: Vec<Ipv4Addr>,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            bind_address: Ipv4Addr::new(127, 0, 0, 1),
            data_dir: "/tmp/virtus".to_string(),
            peers: vec![],
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_dir(mut self, dir: &str) -> Self {
        self.data_dir = dir.to_string();
        self
    }

    pub fn bind(mut self, address: Ipv4Addr) -> Self {
        self.bind_address = address;
        self
    }

    pub fn join_cluster(mut self, peers: Vec<Ipv4Addr>) -> Self {
        self.peers = peers;
        self
    }

    pub fn from_config(id: u32, dir: Option<&str>) -> Self {
        let data_dir = dir.unwrap_or("/tmp/virtus");

        todo!()
    }

    pub fn build(self) -> Result<Virtus, Error> {
        Virtus::new(self.id, self.bind_address, self.data_dir, self.peers)
    }
}
