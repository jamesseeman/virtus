use crate::{Connection, Error, Interface};
use anyhow::Result;
use futures::stream::TryStreamExt;
use netlink_packet_route::link::{LinkAttribute, LinkMessage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    id: Uuid,
    vlan: u32,
    external: bool,
    // Will need to handle uplink being different on different hosts, tuple with node id, int name?
    bridge_name: String,
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

        let bridge_name = match physical_uplink {
            Some(name) => String::from(name),
            None => String::from("virtus-br"),
        };

        if let Some(uplink) = physical_uplink {
            if let Some(_) = Network::get_external_network(uplink, &conn)? {
                return Err(Error::PhysicalNetworkExists.into());
            }
        } else {
            // Make sure that virtus-br exists
            if let None = Network::get_link("virtus-br", &conn).await? {
                Network::create_bridge("virtus-br", &conn).await?;
            }
        }

        let network = Self {
            id: Uuid::new_v4(),
            name: String::from(name),
            vlan: vlan.unwrap_or(0),
            cidr4: cidr4.map(|cidr| String::from(cidr)),
            interfaces: vec![],
            external: physical_uplink.is_some(),
            bridge_name,
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

    pub fn get_bridge_name(&self) -> String {
        self.bridge_name.clone()
    }

    pub fn get_external_network(uplink: &str, conn: &Connection) -> Result<Option<Network>> {
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
            .filter(|network| network.external && network.bridge_name == string_uplink)
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

    pub fn find(name: &str, conn: &Connection) -> Result<Option<Self>> {
        let string_name = String::from(name);
        let mut networks: Vec<Network> = conn
            .db
            .open_tree("networks")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|(_, network)| bincode::deserialize::<Network>(&network).ok())
            .filter(|network| network.name == string_name)
            .collect();

        Ok(networks.pop())
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

    pub async fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        // todo: handle when network (ip link) doesn't exist
        // todo: handle when attached to vm
        if let Some(network) = Network::get(&id, &conn)? {
            for interface in network.interfaces {
                Interface::delete_by_id(interface, &conn).await?;
            }

            conn.db.open_tree("networks")?.remove(id)?;
        }

        Ok(())
    }

    pub async fn delete(self, conn: &Connection) -> Result<()> {
        // todo: handle when network (ip link) doesn't exist
        // todo: handle when attached to vm
        for interface in self.interfaces {
            Interface::delete_by_id(interface, &conn).await?;
        }
        conn.db.open_tree("networks")?.remove(self.id)?;
        Ok(())
    }

    pub async fn get_link(name: &str, conn: &Connection) -> Result<Option<LinkMessage>> {
        let mut links = conn
            .handle
            .link()
            .get()
            .match_name(String::from(name))
            .execute();

        match links.try_next().await {
            Ok(link) => Ok(link),
            Err(_) => Ok(None),
        }
    }

    pub async fn get_link_by_id(id: u32, conn: &Connection) -> Result<Option<LinkMessage>> {
        let mut links = conn.handle.link().get().match_index(id).execute();

        match links.try_next().await {
            Ok(link) => Ok(link),
            Err(_) => Ok(None),
        }
    }

    pub fn get_link_name(link: &LinkMessage) -> Option<String> {
        for attr in link.attributes.iter() {
            if let LinkAttribute::IfName(name) = attr {
                return Some(name.clone());
            }
        }

        None
    }

    pub async fn create_bridge(name: &str, conn: &Connection) -> Result<LinkMessage> {
        conn.handle
            .link()
            .add()
            .bridge(String::from(name))
            .execute()
            .await?;

        let link = Network::get_link(name, conn).await?.unwrap();
        conn.handle
            .link()
            .set(link.header.index)
            .up()
            .execute()
            .await?;

        Ok(link)
    }
}
