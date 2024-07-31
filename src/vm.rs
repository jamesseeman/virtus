use quick_xml::writer::Writer;
use serde::Serialize;

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use std::error::Error;
use std::io::Cursor;
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

#[derive(Debug, Clone)]
pub struct Image {
    id: Uuid,
    installer: bool,
    filename: String,
}

impl Image {
    pub fn new(filename: String, installer: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            filename,
            installer,
        }
    }

    pub fn get_filename(&self) -> String {
        self.filename.clone()
    }

    pub fn is_installer(&self) -> bool {
        self.installer
    }
}

#[derive(Debug, Clone)]
pub struct Network {
    id: Uuid,
    interface_id: String,
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

        let interface_id = id.to_string()[..8].to_string();

        println!("{} {}", id, vlan_id);
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "ovs-vsctl add-port virtus-int {} tag={} -- set interface {} type=internal",
                interface_id, vlan_id, interface_id
            ))
            .output()?;

        if output.status.success() {
            Ok(Self {
                id,
                interface_id,
                vlan: vlan_id,
                name,
                internal,
                cidr4,
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

#[derive(Debug)]
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

impl VM {
    pub fn new(
        name: String,
        cpus: u8,
        memory: u64,
        disk: Disk,
        image: Image,
        network: Network,
    ) -> Self {
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

    pub fn to_xml(&self) -> Result<String, VirtusError> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut domain = BytesStart::new("domain");
        domain.push_attribute(("type", "kvm"));
        writer.write_event(Event::Start(domain))?;

        writer
            .create_element("name")
            .write_text_content(BytesText::new(&self.name.clone()))?;

        writer
            .create_element("uuid")
            .write_text_content(BytesText::new(&self.id.to_string()))?;

        writer
            .create_element("memory")
            .with_attribute(("unit", "MB"))
            .write_text_content(BytesText::new(&self.memory.to_string()))?;

        writer
            .create_element("vcpu")
            .write_text_content(BytesText::new(&self.cpus.to_string()))?;

        writer
            .create_element("os")
            .write_inner_content::<_, VirtusError>(|writer| {
                writer
                    .create_element("type")
                    .with_attributes([("arch", "x86_64"), ("machine", "q35")])
                    .write_text_content(BytesText::new("hvm"))?;

                writer
                    .create_element("boot")
                    .with_attribute(("dev", "hd"))
                    .write_empty()?;

                Ok(())
            })?;

        writer
            .create_element("devices")
            .write_inner_content::<_, VirtusError>(|writer| {
                /*
                                writer
                                    .create_element("disk")
                                    .with_attributes([("type", "file"), ("device", "disk")])
                                    .write_inner_content::<_, VirtusError>(|writer| {
                                        writer
                                            .create_element("driver")
                                            .with_attributes([("name", "qemu"), ("type", "qcow2")])
                                            .write_empty()?;

                                        writer
                                            .create_element("source")
                                            .with_attribute(("file", self.disk.filename.as_str()))
                                            .write_empty()?;

                                        writer
                                            .create_element("target")
                                            .with_attributes([("dev", "vda"), ("bus", "virtio")])
                                            .write_empty()?;

                                        Ok(())
                                    })?;
                */

                writer
                    .create_element("disk")
                    .with_attributes([("type", "file"), ("device", "cdrom")])
                    .write_inner_content::<_, VirtusError>(|writer| {
                        writer
                            .create_element("driver")
                            .with_attributes([("name", "qemu"), ("type", "raw")])
                            .write_empty()?;

                        writer
                            .create_element("source")
                            .with_attribute(("file", self.image.filename.as_str()))
                            .write_empty()?;

                        writer
                            .create_element("target")
                            .with_attributes([("dev", "sda"), ("bus", "sata")])
                            .write_empty()?;

                        Ok(())
                    })?;

                writer
                    .create_element("interface")
                    .with_attribute(("type", "direct"))
                    .write_inner_content::<_, VirtusError>(|writer| {
                        writer
                            .create_element("source")
                            .with_attributes([
                                ("dev", self.network.interface_id.to_string().as_str()),
                                ("mode", "bridge"),
                            ])
                            .write_empty()?;

                        writer
                            .create_element("model")
                            .with_attribute(("type", "virtio"))
                            .write_empty()?;

                        Ok(())
                    })?;

                Ok(())
            })?;

        writer.write_event(Event::End(BytesEnd::new("domain")))?;
        let xml = writer.into_inner().into_inner();
        Ok(String::from_utf8(xml).unwrap())
    }

    pub fn build(&mut self, conn: &virt::connect::Connect) -> Result<(), virt::error::Error> {
        self.domain = Some(virt::domain::Domain::create_xml(
            conn,
            &self.to_xml().unwrap(),
            virt::sys::VIR_DOMAIN_RUNNING,
        )?);

        Ok(())
    }

    pub fn delete(self) -> Result<(), virt::error::Error> {
        if let Some(d) = self.domain {
            return d.destroy();
        }
        Ok(())
    }
}
