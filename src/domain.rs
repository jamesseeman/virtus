//use crate::qcow::Qcow;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "vcpu")]
pub struct CPU {
    #[serde(rename = "$value")]
    pub num: i8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "memory")]
pub struct Memory {
    #[serde(rename = "@unit")]
    pub unit: String,
    #[serde(rename = "$value")]
    pub size: i32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OSBootDev {
    #[default]
    HD,
    FD,
    CDROM,
    Network,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct OSBoot {
    #[serde(rename = "@dev")]
    pub dev: OSBootDev,
}

#[derive(Debug, Clone, Serialize)]
pub struct OSType {
    #[serde(rename = "@arch")]
    pub arch: String,
    #[serde(rename = "$text")]
    pub text: String,
}

impl Default for OSType {
    fn default() -> Self {
        Self {
            arch: "x86_64".into(),
            text: "hvm".into(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct OS {
    pub r#type: OSType,
    pub boot: OSBoot,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainType {
    #[default]
    KVM,
    XEN,
    HVF,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "domain")]
pub struct DomainConfig {
    #[serde(rename = "@type")]
    r#type: DomainType,
    pub name: String,
    pub cpus: CPU,
    pub memory: Memory,
    pub os: OS,
}

impl DomainConfig {
    pub fn new(name: String, cpus: i8, memory: i32) -> Self {
        Self {
            r#type: DomainType::default(),
            name,
            cpus: CPU { num: cpus },
            memory: Memory {
                unit: "MiB".into(),
                size: memory,
            },
            os: OS::default(),
        }
    }

    // add disk

    // remove disk

    // add interface

    // remove interface

    pub fn to_xml(&self) -> Result<String, quick_xml::DeError> {
        quick_xml::se::to_string(self)
    }

    pub fn build(
        &self,
        conn: &virt::connect::Connect,
        flags: u32,
    ) -> Result<Domain, virt::error::Error> {
        Domain::create(conn, self.clone(), flags)
    }
}

#[derive(Debug)]
pub struct Snapshot {}

#[derive(Debug)]
pub enum State {
    STARTED,
    STOPPED,
    PAUSED,
}

#[derive(Debug)]
pub struct Domain {
    config: DomainConfig,
    state: State,
    snapshots: Vec<Snapshot>,
}

impl Domain {
    pub fn create(
        conn: &virt::connect::Connect,
        config: DomainConfig,
        flags: u32,
    ) -> Result<Self, virt::error::Error> {
        virt::domain::Domain::create_xml(conn, &config.to_xml().unwrap(), flags)?;

        Ok(Self {
            config,
            state: State::STOPPED,
            snapshots: vec![],
        })
    }
    // start, stop, migrate, delete, clone
}
