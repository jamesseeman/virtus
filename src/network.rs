use crate::{Connection, Interface};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    id: Uuid,
    vlan: u32,
    name: Option<String>,
    cidr4: Option<String>,
    interfaces: Vec<Uuid>,
}

impl Network {
    pub fn new(
        name: Option<String>,
        vlan: Option<u32>,
        cidr4: Option<String>,
        conn: &Connection,
    ) -> Result<Self> {
        let network = Self {
            id: Uuid::new_v4(),
            name,
            vlan: vlan.unwrap_or(0),
            cidr4,
            interfaces: vec![],
        };
        conn.db
            .open_tree("networks")?
            .insert(network.id, bincode::serialize(&network)?)?;
        Ok(network)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn get_vlan(&self) -> u32 {
        self.vlan
    }

    pub fn set_vlan(&self, vlan: u32) {
        todo!();
    }

    pub fn add_interface(&mut self, id: &Uuid, conn: &Connection) -> Result<()> {
        self.interfaces.push(id.clone());
        conn.db
            .open_tree("networks")?
            .insert(self.id, bincode::serialize(&self)?)?;
        Ok(())
    }

    pub fn remove_interface(&mut self, id: &Uuid, conn: &Connection) -> Result<()> {
        if let Some(index) = self.interfaces.iter().position(|interface_id| interface_id == id) {
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
