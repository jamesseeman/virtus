use crate::{Connection, Disk, Error, Image, Interface, Network};
use anyhow::Result;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum VMState {
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
    // todo replace struct ref with uuid ref?
    pub async fn new(
        name: &str,
        cpus: u8,
        memory: u64,
        size: u64,
        image: &Image,
        network: &mut Network,
        conn: &Connection,
    ) -> Result<Self> {
        if let Some(_) = VM::find(name, conn)? {
            return Err(Error::VMExists.into());
        }

        let disk = Disk::new(size, conn)?;
        let interface = Interface::new(network, conn).await?;
        let vm = Self {
            id: Uuid::new_v4(),
            name: String::from(name),
            cpus,
            memory,
            disks: vec![disk.get_id()],
            image: image.get_id(),
            interfaces: vec![interface.get_id()],
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

    pub async fn to_xml(&self, conn: &Connection) -> Result<String> {
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

        // Grabbing all the interface indices here so I don't have to deal with async closures
        // later
        //
        // todo: make this cleaner
        let mut interfaces = vec![];
        for id in self.interfaces.iter() {
            if let Some(interface) = Interface::get(id, &conn)? {
                let link_index = interface.get_veth_pair().1;
                if let Some(link) = Network::get_link_by_id(link_index, &conn).await? {
                    if let Some(link_name) = Network::get_link_name(&link) {
                        interfaces.push(link_name);
                    }
                }
            }
        }

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
                                .with_attribute(("file", disk.get_filename().as_str()))
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
                if image.is_installer() {
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
                                .with_attribute(("file", image.get_filename().as_str()))
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

                // Bridge NIC
                for interface in interfaces {
                    writer
                        .create_element("interface")
                        .with_attribute(("type", "direct"))
                        .write_inner_content::<_, anyhow::Error>(|writer| {
                            writer
                                .create_element("source")
                                .with_attributes([("dev", interface.as_str()), ("mode", "bridge")])
                                .write_empty()?;

                            writer
                                .create_element("model")
                                .with_attribute(("type", "virtio"))
                                .write_empty()?;

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
                    .write_inner_content::<_, Error>(|writer| {
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
                    .write_inner_content::<_, Error>(|writer| {
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

    pub async fn build(&mut self, conn: &Connection) -> Result<()> {
        let domain = virt::domain::Domain::define_xml(&conn.virt, &self.to_xml(conn).await?)?;
        domain.create()?;
        self.domain = Some(domain);
        Ok(())
    }

    pub fn start(&self) -> Result<()> {
        match self.get_state()? {
            VMState::STOPPED => {
                self.domain.as_ref().unwrap().create()?;
                Ok(())
            }
            VMState::PAUSED => {
                self.domain.as_ref().unwrap().resume()?;
                Ok(())
            }
            VMState::SHUTTING_DOWN => return Err(Error::VMShuttingDown.into()),
            VMState::UNDEFINED => return Err(Error::VMUndefined.into()),
            VMState::RUNNING => Ok(()),
        }
    }

    pub fn get_state(&self) -> Result<VMState> {
        match &self.domain {
            Some(domain) => {
                match domain.get_state()?.0 {
                    // todo: more granular state mgmt
                    virt::sys::VIR_DOMAIN_RUNNING => Ok(VMState::RUNNING),
                    virt::sys::VIR_DOMAIN_PAUSED
                    | virt::sys::VIR_DOMAIN_BLOCKED
                    | virt::sys::VIR_DOMAIN_PMSUSPENDED => Ok(VMState::PAUSED),
                    virt::sys::VIR_DOMAIN_SHUTDOWN => Ok(VMState::SHUTTING_DOWN),
                    // thank you formatter >:(
                    virt::sys::VIR_DOMAIN_SHUTOFF | virt::sys::VIR_DOMAIN_CRASHED => {
                        Ok(VMState::STOPPED)
                    }
                    _ => Ok(VMState::UNDEFINED),
                }
            }
            None => Ok(VMState::UNDEFINED),
        }
    }

    pub async fn delete_by_id(id: Uuid, conn: &Connection) -> Result<()> {
        if let Some(vm) = VM::get(&id, &conn)? {
            match vm.get_state()? {
                // self.domain.unwrap() should be fine, since get_state()
                // returned successfully with VMState
                VMState::RUNNING | VMState::SHUTTING_DOWN | VMState::PAUSED => {
                    let domain = vm.domain.unwrap();
                    domain.destroy()?;
                    domain.undefine()?;
                }
                VMState::STOPPED => vm.domain.unwrap().undefine()?,
                VMState::UNDEFINED => {}
            }

            for disk in vm.disks {
                Disk::delete_by_id(disk, &conn)?;
            }

            for interface in vm.interfaces {
                Interface::delete_by_id(interface, &conn).await?;
            }

            conn.db.open_tree("virtual_machines")?.remove(id)?;
        }

        Ok(())
    }

    pub async fn delete(self, conn: &Connection) -> Result<()> {
        match self.get_state()? {
            // self.domain.unwrap() should be fine, since get_state()
            // returned successfully with VMState
            VMState::RUNNING | VMState::SHUTTING_DOWN | VMState::PAUSED => {
                let domain = self.domain.unwrap();
                domain.destroy()?;
                domain.undefine()?;
            }
            VMState::STOPPED => self.domain.unwrap().undefine()?,
            VMState::UNDEFINED => {}
        }

        for disk in self.disks {
            Disk::delete_by_id(disk, &conn)?;
        }

        for interface in self.interfaces {
            Interface::delete_by_id(interface, &conn).await?;
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

    pub fn list(conn: &Connection) -> Result<Vec<Uuid>> {
        let vms: Vec<Uuid> = conn
            .db
            .open_tree("virtual_machines")?
            .into_iter()
            .filter_map(|result| result.ok())
            .filter_map(|option| Uuid::from_slice(&option.0).ok())
            .collect();

        Ok(vms)
    }
}
