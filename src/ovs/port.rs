use crate::ovs::{Object, interface::Interface};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Port {
    uuid: Option<String>,
    bridge_name: String,
    name: String,
    interfaces: Vec<Interface>
}

impl Port {
    pub fn new(name: String, bridge_name: String) -> Self {
        Self {
            name: name.clone(),
            bridge_name: bridge_name,
            interfaces: vec![Interface::new(name, String::from("internal"))],
            ..Default::default()
        }
    }

    pub fn get_bridge_name(&self) -> String {
        self.bridge_name.clone()
    }
}

impl Object for Port {
    fn get_table(&self) -> String {
        String::from("Port")
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_uuid(&self) -> Option<String> {
        self.uuid.clone()
    }

    fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid)
    }
}
