use crate::{Connection, Error, Network};
use anyhow::Result;
use netlink_packet_route::link::LinkMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Interface {
    id: Uuid,
    network: Uuid,
    mac_addr: Option<[u8; 6]>,
    veth_pair: (u32, u32),
}

impl Interface {
    pub async fn new(network: &mut Network, conn: &Connection) -> Result<Self> {
        let id = Uuid::new_v4();
        let link_name = id.to_string()[..8].to_string();

        let (link1, link2) = Interface::create_link(
            link_name.as_str(),
            network.get_bridge_name().as_str(),
            &conn,
        )
        .await?;

        let interface = Self {
            id,
            network: network.get_id(),
            mac_addr: None,
            veth_pair: (link1.header.index, link2.header.index),
        };

        network.add_interface(&id, &conn)?;
        conn.db
            .open_tree("interfaces")?
            .insert(id, bincode::serialize(&interface)?)?;
        Ok(interface)
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_network(&self, conn: &Connection) -> Result<Network> {
        Ok(Network::get(&self.network, &conn)?.unwrap())
    }

    pub fn get_network_id(&self) -> Uuid {
        self.network
    }

    pub fn get_veth_pair(&self) -> (u32, u32) {
        self.veth_pair
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

    pub async fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        // todo: handle when attached to vm
        if let Some(interface) = Interface::get(&id, &conn)? {
            conn.handle
                .link()
                .del(interface.veth_pair.0)
                .execute()
                .await?;
            conn.handle
                .link()
                .del(interface.veth_pair.1)
                .execute()
                .await?;

            let mut network = Network::get(&interface.network, &conn)?.unwrap();
            network.remove_interface(&interface.id, &conn)?;
            conn.db.open_tree("interfaces")?.remove(id)?;
        }

        Ok(())
    }

    pub async fn delete(self, conn: &Connection) -> Result<()> {
        // todo: handle when attached to vm
        conn.handle.link().del(self.veth_pair.0).execute().await?;
        conn.handle.link().del(self.veth_pair.1).execute().await?;

        let mut network = Network::get(&self.network, &conn)?.unwrap();
        network.remove_interface(&self.id, &conn)?;
        conn.db.open_tree("interfaces")?.remove(self.id)?;
        Ok(())
    }

    pub async fn create_link(
        name: &str,
        bridge_name: &str,
        conn: &Connection,
    ) -> Result<(LinkMessage, LinkMessage)> {
        match Network::get_link(bridge_name, &conn).await? {
            Some(bridge) => {
                conn.handle
                    .link()
                    .add()
                    .veth(format!("{}-1", name), format!("{}-2", name))
                    .execute()
                    .await?;

                let link1 = Network::get_link(format!("{}-1", name).as_str(), &conn)
                    .await?
                    .unwrap();
                let link2 = Network::get_link(format!("{}-2", name).as_str(), &conn)
                    .await?
                    .unwrap();

                conn.handle
                    .link()
                    .set(link1.header.index)
                    .up()
                    .controller(bridge.header.index)
                    .execute()
                    .await?;

                Ok((link1, link2))
            }
            None => Err(Error::InterfaceNotFound.into()),
        }
    }
}
