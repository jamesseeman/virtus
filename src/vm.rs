use serde::Serialize;
use std::error::Error;
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Disk {
    id: Uuid,
    filename: String,
    size: usize,
}

impl Disk {
    pub fn create(size: usize) -> Result<Self, Box<dyn Error>> {
        let id = Uuid::new_v4();
        //        let filename = format!("/var/lib/virtus/{}.qcow2", id);
        let filename = format!("/tmp/{}.qcow2", id);

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("qemu-img create {} {}", filename, size))
            .output()?;

        println!("{}", String::from_utf8(output.stdout).unwrap());

        if output.status.success() {
            return Ok(Self { id, filename, size });
        } else {
            println!("{}", String::from_utf8(output.stderr).unwrap());
            return Err("failed to provision disk".into());
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
    cpus: i8,
    disk: Disk,
    image: Image,
    network: Network,
    domain: Option<virt::domain::Domain>,
    state: State,
}

impl VM {
    pub fn new(cpus: i8, disk: Disk, image: Image, network: Network) -> Self {
        Self {
            id: Uuid::new_v4(),
            cpus: cpus,
            disk: disk,
            image: image,
            network: network,
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
