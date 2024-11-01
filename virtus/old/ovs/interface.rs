use crate::ovs::{port::Port, Object};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Interface {
    uuid: Option<String>,
    name: String,
    r#type: String,
}

impl Interface {
    pub fn new(name: String, r#type: String) -> Self {
        Self {
            name: name,
            r#type: r#type,
            ..Default::default()
        }
    }

    /*
    pub fn get_port(&self) -> Port {
        self.port.clone()
    }
    */
}

impl Object for Interface {
    fn get_table(&self) -> String {
        String::from("Interface")
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
