use serde::Serialize;
use serde::ser::{SerializeMap, SerializeStruct};
use std::error::Error;
use std::process::Command;
use uuid::Uuid;

use crate::error::VirtusError;

#[derive(Debug, Clone)]
pub struct Disk {
    id: Uuid,
    filename: String,
    size: usize,
}

impl Disk {
    pub fn create(size: usize) -> Result<Self, VirtusError> {
        let id = Uuid::new_v4();
        //        let filename = format!("/var/lib/virtus/{}.qcow2", id);
        // todo: /var/lib/virtus
        let filename = format!("/tmp/{}.qcow2", id);

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("qemu-img create {} {}", filename, size))
            .output()?;

        println!("{}", String::from_utf8(output.stdout).unwrap());

        if output.status.success() {
            Ok(Self { id, filename, size })
        } else {
            println!("{}", String::from_utf8(output.stderr).unwrap());
            Err(VirtusError::DiskError)
        }
    }

    pub fn delete(self) -> Result<String, std::io::Error> {
        std::fs::remove_file(&self.filename)?;

        Ok(self.filename.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Image {
    id: Uuid,
    installer: bool,
    file: String,
}

impl Image {
    pub fn new(file: String, installer: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            file,
            installer,
        }
    }

    pub fn get_file(&self) -> String {
        self.file.clone()
    }

    pub fn is_installer(&self) -> bool {
        self.installer
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Network {
    id: Uuid,
    vlan: i32,
    name: Option<String>,
    internal: bool,
    cidr4: Option<String>,
}

impl Network {
    pub fn new(
        vlan: Option<i32>,
        name: Option<String>,
        internal: bool,
        cidr4: Option<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let id = Uuid::new_v4();
        let vlan_id = vlan.unwrap_or(0);

        println!("{} {}", id, vlan_id);
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ovs-vsctl add-port virtus-int {} tag={} -- set interface {} type=internal",
                id, vlan_id, id
            ))
            .output()?;

        if output.status.success() {
            Ok(Self {
                id: id,
                vlan: vlan_id,
                name: name,
                internal: internal,
                cidr4: cidr4,
            })
        } else {
            println!("{}", String::from_utf8(output.stderr).unwrap());
            return Err("failed to create network".into());
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn get_vlan(&self) -> i32 {
        self.vlan
    }

    pub fn set_vlan(&self, vlan: i32) {
        todo!();
    }
}

#[derive(Debug, Serialize)]
pub enum State {
    NONE,
    RUNNING,
    STOPPED,
    PAUSED,
}

#[derive(Debug)]
pub struct VM {
    id: Uuid,
    name: String,
    cpus: u8,
    memory: u64,
    disk: Disk,
    image: Image,
    network: Network,
    domain: Option<virt::domain::Domain>,
    state: State,
}

struct Memory(u64);

impl Serialize for Memory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {

        let mut memory = serializer.serialize_map(Some(2))?;
        memory.serialize_entry("@unit", "MB")?;
        memory.serialize_entry("$value", &self.0)?;
        memory.end()
    }
}

enum Device {
    Disk(Disk),
}

impl Serialize for VM {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Does this number need to be accurate?
        let mut domain = serializer.serialize_struct("domain", 1)?;

        // @attr makes it a tag 
        // $value makes it the content
        domain.serialize_field("@type", "kvm")?;
        domain.serialize_field("name", &self.name)?;
        domain.serialize_field("uuid", &self.id.to_string())?;
        domain.serialize_field("vcpu", &self.cpus)?;
        domain.serialize_field("memory", &Memory(self.memory))?;

        domain.end()
    }
}

impl VM {
    pub fn new(name: String, cpus: u8, memory: u64, disk: Disk, image: Image, network: Network) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            cpus,
            memory,
            disk,
            image,
            network,
            domain: None,
            state: State::NONE,
        }
    }

    pub fn to_xml(&self) -> Result<String, virt::error::Error> {
        Ok(String::from("test"))
    }

    pub fn build(
        &self,
        conn: &virt::connect::Connect,
    ) -> Result<virt::domain::Domain, virt::error::Error> {
        virt::domain::Domain::create_xml(
            conn,
            &self.to_xml().unwrap(),
            virt::sys::VIR_DOMAIN_RUNNING,
        )
    }

    pub fn delete(self) -> Result<(), virt::error::Error> {
        if let Some(d) = self.domain {
            return d.destroy();
        }
        Ok(())
    }
}
