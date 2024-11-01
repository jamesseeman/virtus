use crate::ovs::Object;

#[derive(Default, Debug)]
pub struct Bridge {
    uuid: Option<String>,
    name: String,
}

impl Bridge {
    pub fn new(name: String) -> Self {
        Self {
            name: name,
            ..Default::default()
        }
    }
}

impl Object for Bridge {
    fn get_table(&self) -> String {
        String::from("Bridge")
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_uuid(&self) -> Option<String> {
        self.uuid.clone()
    }

    fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid);
    }
}
