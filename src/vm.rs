use quick_xml::writer::Writer;
use serde::{Deserialize, Serialize};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

use crate::{Connection, VirtusError};
use anyhow::Result;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Disk {
    id: Uuid,
    filename: String,
    size: u64,
}

impl Disk {
    pub fn new(size: u64, conn: &Connection) -> Result<Self> {
        let disk_dir = format!("{}/disks", &conn.data_dir);
        if !Path::exists(Path::new(&disk_dir)) {
            fs::create_dir(&disk_dir)?;
        }

        let id = Uuid::new_v4();
        let filename = format!("{}/{}.qcow2", disk_dir, id);

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("qemu-img create -f qcow2 {} {}", filename, size))
            .output()?;

        if output.status.success() {
            let disk = Self { id, filename, size };
            conn.db
                .open_tree("disks")?
                .insert(id, bincode::serialize(&disk)?)?;
            Ok(disk)
        } else {
            Err(VirtusError::DiskError.into())
        }
    }

    pub fn delete(self, conn: &Connection) -> Result<String> {
        // todo: check if a VM is using disk
        // todo: handle when file has already been deleted
        std::fs::remove_file(&self.filename)?;
        conn.db.open_tree("disks")?.remove(self.id)?;
        Ok(self.filename.clone())
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("disks")?.get(id)? {
            Some(disk) => Ok(Some(bincode::deserialize(&disk)?)),
            None => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    id: Uuid,
    installer: bool,
    filename: String,
}

impl Image {
    pub fn new(filename: String, installer: bool, conn: &Connection) -> Result<Self> {
        let image = Self {
            id: Uuid::new_v4(),
            filename,
            installer,
        };
        conn.db
            .open_tree("images")?
            .insert(image.id, bincode::serialize(&image)?)?;
        Ok(image)
    }

    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    pub fn is_installer(&self) -> bool {
        self.installer
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("images")?.get(id)? {
            Some(image) => Ok(Some(bincode::deserialize(&image)?)),
            None => Ok(None),
        }
    }

    //todo delete
}

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

    // todo: delete

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("networks")?.get(id)? {
            Some(network) => Ok(Some(bincode::deserialize(&network)?)),
            None => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Interface {
    id: Uuid,
    network: Uuid,
    mac_addr: Option<[u8; 6]>,
    link_name: String,
}

impl Interface {
    pub fn new(network: &mut Network, conn: &Connection) -> Result<Self> {
        let id = Uuid::new_v4();
        let link_name = id.to_string()[..8].to_string();
        let interface = Self {
            id,
            network: network.id,
            mac_addr: None,
            link_name: link_name.clone(),
        };

        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ovs-vsctl add-port virtus-int {} tag={} -- set interface {} type=internal",
                link_name, network.vlan, link_name
            ))
            .output()?;

        if output.status.success() {
            network.interfaces.push(id);
            conn.db
                .open_tree("interfaces")?
                .insert(id, bincode::serialize(&interface)?)?;
            Ok(interface)
        } else {
            Err(VirtusError::OVSError.into())
        }
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Interface>> {
        match conn.db.open_tree("interfaces")?.get(id)? {
            Some(interface) => Ok(Some(bincode::deserialize(&interface)?)),
            None => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum State {
    UNDEFINED,
    RUNNING,
    SHUTTING_DOWN,
    STOPPED,
    PAUSED,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VM {
    id: Uuid,
    name: String,
    cpus: u8,
    memory: u64,
    disks: Vec<Uuid>,
    image: Uuid,
    interfaces: Vec<Uuid>,
    #[serde(skip)]
    domain: Option<virt::domain::Domain>,
}

impl VM {
    // todo replace struct ref with uuid (ref?)
    pub fn new(
        name: &str,
        cpus: u8,
        memory: u64,
        size: u64,
        image: &Image,
        network: &mut Network,
        conn: &Connection,
    ) -> Result<Self> {
        if let Some(_) = VM::find(name, conn)? {
            return Err(VirtusError::VMExists.into());
        }

        let disk = Disk::new(size, conn)?;
        let interface = Interface::new(network, conn)?;
        let vm = Self {
            id: Uuid::new_v4(),
            name: String::from(name),
            cpus,
            memory,
            disks: vec![disk.id],
            image: image.id,
            interfaces: vec![interface.id],
            domain: None,
        };
        conn.db
            .open_tree("virtual_machines")?
            .insert(vm.id, bincode::serialize(&vm)?)?;
        Ok(vm)
    }

    pub fn attach_network(&self, network: &Network) -> Result<Interface> {
        todo!();
    }

    pub fn attach_disk(&self, disk: &Disk) -> Result<()> {
        todo!();
    }

    pub fn add_disk(&self, size: u64, image: Option<Image>) -> Result<Disk> {
        todo!();
    }

    pub fn detach_disk(&self, disk: &Disk, delete: bool) -> Result<()> {
        todo!();
    }

    pub fn detach_network(&self, network: &Network) -> Result<()> {
        todo!();
    }

    pub fn to_xml(&self, conn: &Connection) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // <domain type=kvm>
        let mut domain = BytesStart::new("domain");
        domain.push_attribute(("type", "kvm"));
        writer.write_event(Event::Start(domain))?;

        // <name>
        writer
            .create_element("name")
            .write_text_content(BytesText::new(&self.name))?;

        // <uuid>
        writer
            .create_element("uuid")
            .write_text_content(BytesText::new(&self.id.to_string()))?;

        // <memory unit=bytes>
        writer
            .create_element("memory")
            .with_attribute(("unit", "bytes"))
            .write_text_content(BytesText::new(&self.memory.to_string()))?;

        // <vcpu>
        writer
            .create_element("vcpu")
            .write_text_content(BytesText::new(&self.cpus.to_string()))?;

        // <os>
        //  <type arch=x86_64 machine=q35>hvm</type>
        // </os>
        writer
            .create_element("os")
            .write_inner_content::<_, anyhow::Error>(|writer| {
                writer
                    .create_element("type")
                    .with_attributes([("arch", "x86_64"), ("machine", "q35")])
                    .write_text_content(BytesText::new("hvm"))?;

                /*
                writer
                    .create_element("boot")
                    .with_attribute(("dev", "hd"))
                    .write_empty()?;
                */

                Ok(())
            })?;

        // <devices>
        writer
            .create_element("devices")
            .write_inner_content::<_, anyhow::Error>(|writer| {
                for disk_id in &self.disks {
                    let disk = Disk::get(disk_id, conn)?.unwrap();

                    // <disk type=file device=disk>
                    //  <driver name=qemu type=qcow2 />
                    writer
                        .create_element("disk")
                        .with_attributes([("type", "file"), ("device", "disk")])
                        .write_inner_content::<_, anyhow::Error>(|writer| {
                            writer
                                .create_element("driver")
                                .with_attributes([("name", "qemu"), ("type", "qcow2")])
                                .write_empty()?;

                            writer
                                .create_element("source")
                                .with_attribute(("file", disk.filename.as_str()))
                                .write_empty()?;

                            writer
                                .create_element("target")
                                .with_attributes([("dev", "vda"), ("bus", "virtio")])
                                .write_empty()?;

                            Ok(())
                        })?;
                }

                // Installer CDROM
                let image = Image::get(&self.image, conn)?.unwrap();
                if image.installer {
                    writer
                        .create_element("disk")
                        .with_attributes([("type", "file"), ("device", "cdrom")])
                        .write_inner_content::<_, anyhow::Error>(|writer| {
                            writer
                                .create_element("driver")
                                .with_attributes([("name", "qemu"), ("type", "raw")])
                                .write_empty()?;

                            writer
                                .create_element("source")
                                .with_attribute(("file", image.filename.as_str()))
                                .write_empty()?;

                            writer
                                .create_element("target")
                                .with_attributes([("dev", "sda"), ("bus", "sata")])
                                .write_empty()?;

                            writer
                                .create_element("boot")
                                .with_attribute(("order", "1"))
                                .write_empty()?;

                            Ok(())
                        })?;
                }

                // OVS NIC
                for interface_id in &self.interfaces {
                    let interface = Interface::get(&interface_id, conn)?.unwrap();
                    writer
                        .create_element("interface")
                        .with_attribute(("type", "direct"))
                        .write_inner_content::<_, VirtusError>(|writer| {
                            writer
                                .create_element("source")
                                .with_attributes([
                                    //                                    ("dev", interface.link_name.as_str()),
                                    ("dev", "vlan.20"),
                                    ("mode", "passthrough"),
                                ])
                                .write_empty()?;

                            writer
                                .create_element("model")
                                .with_attribute(("type", "virtio"))
                                .write_empty()?;

                            /*
                            writer
                                .create_element("vlan")
                                .write_inner_content::<_, VirtusError>(|writer| {
                                    writer
                                        .create_element("tag")
                                        .with_attribute(("id", "20"))
                                        .write_empty()?;

                                    Ok(())
                                })?;
                            */

                            Ok(())
                        })?;
                }

                // <console type=pty />
                writer
                    .create_element("console")
                    .with_attribute(("type", "pty"))
                    .write_empty()?;

                // <table type=tablet bus=usb /> (provides absolute cursor movement)
                writer
                    .create_element("input")
                    .with_attributes([("type", "tablet"), ("bus", "usb")])
                    .write_empty()?;

                // <graphics type=spice port=-1 tlsPort=-1 autoport=yes> (graphics device)
                //  <image compression=off />
                // </graphics>
                writer
                    .create_element("graphics")
                    .with_attributes([
                        ("type", "spice"),
                        ("port", "-1"),
                        ("tlsPort", "-1"),
                        ("autoport", "yes"),
                    ])
                    .write_inner_content::<_, VirtusError>(|writer| {
                        writer
                            .create_element("image")
                            .with_attribute(("compression", "off"))
                            .write_empty()?;

                        Ok(())
                    })?;

                // <rng model=virtio>
                //  <backend model=random>/dev/urandom</backend>
                // </rng>
                writer
                    .create_element("rng")
                    .with_attribute(("model", "virtio"))
                    .write_inner_content::<_, VirtusError>(|writer| {
                        writer
                            .create_element("backend")
                            .with_attribute(("model", "random"))
                            .write_text_content(BytesText::new("/dev/urandom"))?;

                        Ok(())
                    })?;

                Ok(())
            })?;

        // </domain>
        writer.write_event(Event::End(BytesEnd::new("domain")))?;
        let xml = writer.into_inner().into_inner();
        Ok(String::from_utf8(xml).unwrap())
    }

    pub fn build(&mut self, conn: &Connection) -> Result<()> {
        let domain = virt::domain::Domain::define_xml(&conn.virt, &self.to_xml(conn)?)?;
        domain.create()?;
        self.domain = Some(domain);
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        match self.get_state()? {
            State::STOPPED => {
                self.domain.as_ref().unwrap().create()?;
                Ok(())
            }
            State::PAUSED => {
                self.domain.as_ref().unwrap().resume()?;
                Ok(())
            }
            State::SHUTTING_DOWN => return Err(VirtusError::VMShuttingDown.into()),
            State::UNDEFINED => return Err(VirtusError::VMUndefined.into()),
            State::RUNNING => Ok(()),
        }
    }

    pub fn get_state(&self) -> Result<State> {
        match &self.domain {
            Some(domain) => {
                match domain.get_state()?.0 {
                    // todo: more granular state mgmt
                    virt::sys::VIR_DOMAIN_RUNNING => Ok(State::RUNNING),
                    virt::sys::VIR_DOMAIN_PAUSED
                    | virt::sys::VIR_DOMAIN_BLOCKED
                    | virt::sys::VIR_DOMAIN_PMSUSPENDED => Ok(State::PAUSED),
                    virt::sys::VIR_DOMAIN_SHUTDOWN => Ok(State::SHUTTING_DOWN),
                    // thank you formatter >:(
                    virt::sys::VIR_DOMAIN_SHUTOFF | virt::sys::VIR_DOMAIN_CRASHED => {
                        Ok(State::STOPPED)
                    }
                    _ => Ok(State::UNDEFINED),
                }
            }
            None => Ok(State::UNDEFINED),
        }
    }

    pub fn delete(self, conn: &Connection) -> Result<()> {
        match self.get_state()? {
            // self.domain.unwrap() should be fine, since get_state()
            // returned successfully with State
            State::RUNNING | State::SHUTTING_DOWN | State::PAUSED => {
                let domain = self.domain.unwrap();
                domain.destroy()?;
                domain.undefine()?;
            }
            State::STOPPED => self.domain.unwrap().undefine()?,
            State::UNDEFINED => {}
        }

        for disk_id in self.disks {
            if let Some(disk) = Disk::get(&disk_id, &conn)? {
                disk.delete(&conn)?;
            }
        }

        conn.db.open_tree("virtual_machines")?.remove(self.id)?;
        Ok(())
    }

    pub fn get(id: &Uuid, conn: &Connection) -> Result<Option<Self>> {
        match conn.db.open_tree("virtual_machines")?.get(id)? {
            Some(found) => {
                let mut vm: VM = bincode::deserialize(&found)?;
                let mut vm_buf: Vec<virt::domain::Domain> = conn
                    .virt
                    .list_all_domains(
                        virt::sys::VIR_CONNECT_LIST_DOMAINS_ACTIVE
                            | virt::sys::VIR_CONNECT_LIST_DOMAINS_INACTIVE,
                    )?
                    .into_iter()
                    .filter(|domain| domain.get_uuid_string().unwrap() == vm.id.to_string())
                    .collect();

                vm.domain = vm_buf.pop();
                Ok(Some(vm))
            }
            None => Ok(None),
        }
    }

    pub fn find(name: &str, conn: &Connection) -> Result<Option<Self>> {
        // Todo: store vm name somewhere so we don't need to load each vm into memory
        // Probably should store (id, name) as key
        for result in conn.db.open_tree("virtual_machines")?.into_iter() {
            let (_, found) = result?;
            let vm: VM = bincode::deserialize(&found)?;
            if vm.name == name {
                return VM::get(&vm.id, conn);
            }
        }

        Ok(None)
    }
}
