use serde::{Deserialize, Serialize};

use crate::Error;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

pub mod bridge;
pub mod interface;
pub mod port;
pub mod request;
pub mod response;

use crate::{ovs::bridge::Bridge, ovs::port::Port, ovs::request::*, ovs::response::*};

// See RFC 7047 https://www.rfc-editor.org/rfc/rfc7047 for details on the OVS JSON-RPC protocol
#[derive(Default, Debug)]
pub struct Ovs {
    socket: std::path::PathBuf,
    stream: Option<UnixStream>,
    db: String,
    msg_id: i32, // TODO: address overflow
}

/*
*
* TODO instead of object trait, use Record enum

#[derive(Debug)]
pub enum Record {
    Bridge(Bridge),
    Port(Port),
//    Interface(Interface),
}

impl Record {

}

*/

trait Object {
    fn get_table(&self) -> String;
    fn get_name(&self) -> String;
    fn get_uuid(&self) -> Option<String>;
    fn set_uuid(&mut self, uuid: String);
}

impl Ovs {
    pub fn new() -> Self {
        Self {
            socket: "/var/run/openvswitch/db.sock".into(),
            db: String::from("Open_vSwitch"),
            ..Default::default()
        }
    }

    pub fn get_db(&self) -> &String {
        &self.db
    }

    pub fn db(mut self, db: String) -> Self {
        self.db = db;
        self
    }

    pub fn get_socket(&self) -> &std::path::PathBuf {
        &self.socket
    }

    pub fn socket(mut self, socket: std::path::PathBuf) -> Self {
        self.socket = socket;
        self
    }

    pub fn connect(mut self) -> Result<Self, Error> {
        let stream = UnixStream::connect(&self.socket)?;
        self.stream = Some(stream);

        Ok(self)
    }

    pub fn rpc_response(&mut self, request: Request) -> Result<Response, Error> {
        // Convert the struct to json
        let msg = serde_json::to_vec(&request)?;
        //println!("{}", String::from_utf8_lossy(&msg));

        // Send the message to the socket
        self.stream.as_mut().unwrap().write_all(&msg)?;

        let mut buffer = Vec::new();
        let mut scratch = [0; 512];

        loop {
            let response = self.stream.as_mut().unwrap().read(&mut scratch)?;
            buffer.extend_from_slice(&scratch[..response]);

            if response != scratch.len() {
                break;
            }
        }

        self.msg_id += 1;

        Response::try_from(buffer)
    }

    pub fn list_dbs(&mut self) -> Result<Vec<Entry>, Error> {
        let request = Request::new(Method::ListDbs, self);
        let response = self.rpc_response(request)?;
        Ok(response.result)
    }

    pub fn create<T: Object>(&mut self, obj: &mut T) -> Result<String, Error> {
        let request = Request::new(Method::Transact, self).insert(obj);
        let response = self.rpc_response(request)?;

        match &response.result[0] {
            Entry::Uuid { uuid } => match uuid {
                Some(uuid) => {
                    obj.set_uuid(uuid.1.clone());
                    Ok(uuid.1.clone())
                }
                None => Err(Error::DbError),
            },
            _ => Err(Error::DbError),
        }
    }

    pub fn delete<T: Object>(&mut self, obj: T) -> Result<(), Error> {
        let request = Request::new(Method::Transact, self).delete(&obj);
        let _ = self.rpc_response(request)?;
        Ok(())
    }

    pub fn get_bridges(&mut self) -> Result<Vec<Bridge>, Error> {
        let request = Request::new(Method::Transact, self).select(String::from("Bridge"));
        let response = self.rpc_response(request)?;

        match &response.result[0] {
            Entry::Rows { rows } => {
                let bridges = rows
                    .iter()
                    .map(|row| {
                        let mut bridge = Bridge::new(row.name.clone());
                        bridge.set_uuid(row.uuid.1.clone());
                        bridge
                    })
                    .collect();
                Ok(bridges)
            }
            _ => Err(Error::DbError),
        }
    }

    pub fn find_bridge(&mut self, name: String) -> Result<Option<Bridge>, Error> {
        let bridges = self.get_bridges()?;
        Ok(bridges.into_iter().find(|bridge| bridge.get_name() == name))
    }

    pub fn get_ports(&mut self) -> Result<Vec<Entry>, Error> {
        let request = Request::new(Method::Transact, self).select(String::from("Port"));
        let response = self.rpc_response(request)?;

        /*
                match &response.result[0] {
                    Entry::Rows { rows } => {
                        let ports = rows
                            .iter()
                            .map(|row| {
                                let mut port = Port::new(row.name.clone(), );
                                bridge.set_uuid(row.uuid.1.clone());
                                bridge
                            })
                            .collect();
                        Ok(bridges)
                    }
                    _ => Err(Error::DbError),
                }
        */
        Ok(response.result)
    }

    pub fn get_interfaces(&mut self) -> Result<Vec<Entry>, Error> {
        let request = Request::new(Method::Transact, self).select(String::from("Interface"));
        let response = self.rpc_response(request)?;
        Ok(response.result)
    }
}
