use crate::{Connection, Error, Network};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Interface {
    id: Uuid,
    network: Uuid,
    mac_addr: Option<[u8; 6]>,
    link_name: String,
}

impl Interface {
    // todo: persist interface <-> network vector to kv store
    pub fn new(network: &mut Network, conn: &Connection) -> Result<Self> {
        let id = Uuid::new_v4();
        let link_name = id.to_string()[..8].to_string();
        let interface = Self {
            id,
            network: network.get_id(),
            mac_addr: None,
            link_name: link_name.clone(),
        };

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ovs-vsctl add-port virtus-int {} tag={} -- set interface {} type=internal",
                link_name,
                network.get_vlan(),
                link_name
            ))
            .output()?;

        if output.status.success() {
            network.add_interface(&id, &conn)?;
            conn.db
                .open_tree("interfaces")?
                .insert(id, bincode::serialize(&interface)?)?;
            Ok(interface)
        } else {
            Err(Error::OVSError.into())
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Interface>> {
        match conn.db.open_tree("interfaces")?.get(id)? {
            Some(interface) => Ok(Some(bincode::deserialize(&interface)?)),
            None => Ok(None),
        }
    }

    pub fn list(conn: &Connection) -> Result<Vec<Uuid>> {
        let interfaces: Vec<Uuid> = conn
            .db
            .open_tree("interfaces")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|option| Uuid::from_slice(&option.0).ok())
            .collect();

        Ok(interfaces)
    }

    pub fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        // todo: update parent network interfaces
        conn.db.open_tree("interfaces")?.remove(id)?;
        Ok(())
    }

    pub fn delete(self, conn: &Connection) -> Result<()> {
        // todo: update parent network interfaces
        conn.db.open_tree("interfaces")?.remove(self.id)?;
        Ok(())
    }
}
