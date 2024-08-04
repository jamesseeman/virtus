use crate::{Connection, Error, Interface};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    id: Uuid,
    vlan: u32,
    // Will need to handle uplink being different on different hosts, tuple with node id, int name?
    physical_uplink: Option<String>,
    name: String,
    cidr4: Option<String>,
    interfaces: Vec<Uuid>,
}

impl Network {
    pub async fn new(
        name: &str,
        vlan: Option<u32>,
        cidr4: Option<&str>,
        physical_uplink: Option<&str>,
        conn: &Connection,
    ) -> Result<Self> {
        let tree = conn.db.open_tree("networks")?;

        if let Some(uplink) = physical_uplink {
            if let Some(_) = Network::get_network_by_uplink(uplink, &conn)? {
                return Err(Error::PhysicalNetworkExists.into());
            }
        } /*else {
            // Make sure that virtus-br exists

        } */

        let network = Self {
            id: Uuid::new_v4(),
            name: String::from(name),
            vlan: vlan.unwrap_or(0),
            cidr4: cidr4.map(|cidr| String::from(cidr)),
            interfaces: vec![],
            physical_uplink: physical_uplink.map(|uplink| String::from(uplink)),
        };

        tree.insert(network.id, bincode::serialize(&network)?)?;

        Ok(network)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_vlan(&self) -> u32 {
        self.vlan
    }

    pub fn set_vlan(&self, vlan: u32) {
        todo!();
    }

    pub fn get_physical_uplink(&self) -> Option<String> {
        self.physical_uplink.clone()
    }

    pub fn get_network_by_uplink(uplink: &str, conn: &Connection) -> Result<Option<Network>> {
        let string_uplink = String::from(uplink);

        // Iterate through each network, convert to struct, grab physical uplink, compare to
        // given uplink.
        // Probably should replace with for loop to avoid loading all networks into memory
        let mut networks: Vec<Network> = conn
            .db
            .open_tree("networks")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|(_, network)| bincode::deserialize::<Network>(&network).ok())
            .filter(|network| {
                network
                    .physical_uplink
                    .as_ref()
                    .is_some_and(|upl| upl == &string_uplink)
            })
            .collect();

        Ok(networks.pop())
    }

    pub fn add_interface(&mut self, id: &Uuid, conn: &Connection) -> Result<()> {
        self.interfaces.push(id.clone());
        conn.db
            .open_tree("networks")?
            .insert(self.id, bincode::serialize(&self)?)?;
        Ok(())
    }

    pub fn remove_interface(&mut self, id: &Uuid, conn: &Connection) -> Result<()> {
        if let Some(index) = self
            .interfaces
            .iter()
            .position(|interface_id| interface_id == id)
        {
            self.interfaces.remove(index);
            conn.db
                .open_tree("networks")?
                .insert(self.id, bincode::serialize(&self)?)?;
        }
        Ok(())
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("networks")?.get(id)? {
            Some(network) => Ok(Some(bincode::deserialize(&network)?)),
            None => Ok(None),
        }
    }

    pub fn list(conn: &Connection) -> Result<Vec<Uuid>> {
        let networks: Vec<Uuid> = conn
            .db
            .open_tree("networks")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|option| Uuid::from_slice(&option.0).ok())
            .collect();

        Ok(networks)
    }

    pub fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        // todo: handle when network (ip link) doesn't exist
        // todo: handle when attached to vm
        if let Some(network) = Network::get(&id, &conn)? {
            for interface in network.interfaces {
                Interface::delete_by_id(interface, &conn)?;
            }

            conn.db.open_tree("networks")?.remove(id)?;
        }

        Ok(())
    }

    pub fn delete(self, conn: &Connection) -> Result<()> {
        // todo: handle when network (ip link) doesn't exist
        // todo: handle when attached to vm
        for interface in self.interfaces {
            Interface::delete_by_id(interface, &conn)?;
        }
        conn.db.open_tree("networks")?.remove(self.id)?;
        Ok(())
    }
}
