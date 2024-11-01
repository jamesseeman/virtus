use serde::Deserialize;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

/*
* TODO: make more functional
*
* let ovs = ovs::new().connect("/var/run/openvswitch/ovsdb.sock")
* let bridge = ovs::Bridge{
*   name='test'
* }
*
* ovs.create(bridge) -> Result<Row>
* ovs.find("asdf") -> Result<Struct>
* ovs.delete(bridge) -> Result<>
*
*/

// See RFC 7047 https://www.rfc-editor.org/rfc/rfc7047 for details on the OVS JSON-RPC protocol
pub struct Ovs {
    socket: std::path::PathBuf,
    stream: UnixStream,
    db: String,
    msg_id: i32, // TODO: address overflow
}

#[derive(Deserialize, Debug, Clone)]
pub struct OvsResult {
    id: i32,
    result: Vec<OvsResultEntry>,
    error: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OvsResultEntry {
    uuid: Option<(String, String)>,
}

impl Ovs {
    pub fn new(socket: std::path::PathBuf, db: Option<String>) -> Result<Self, std::io::Error> {
        let stream = UnixStream::connect(&socket)?;
        Ok(Self {
            socket: socket,
            stream: stream,

            // We're keeping the default 'Open_vSwitch' database, we may want to use a dedicated
            // virtus database in the future
            db: db.unwrap_or("Open_vSwitch".into()),
            msg_id: 0,
        })
    }

    // Write a message to the ovsdb socket and return the response
    pub fn send_message(&mut self, msg: &str) -> Result<Vec<u8>, std::io::Error> {
        self.stream.write_all(msg.as_bytes())?;

        let mut buffer = Vec::new();
        let mut scratch = [0; 512];

        loop {
            let response = self.stream.read(&mut scratch)?;
            buffer.extend_from_slice(&scratch[..response]);

            if response != scratch.len() {
                break;
            }
        }

        Ok(buffer)
    }

    // Get the conf db
    pub fn get_db(&self) -> String {
        self.db.clone()
    }

    pub fn list_dbs(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let msg = format!(
            "{}{}{}",
            r#"{"method": "list_dbs", "id": "#, self.msg_id, r#", "params": []}"#
        );
        self.msg_id += 1;
        self.send_message(msg.as_ref())
    }

    // Select all rows in a table from self.db
    // See https://www.man7.org/linux/man-pages/man5/ovs-vswitchd.conf.db.5.html#TABLE_SUMMARY
    // ex. Bridge, Port, Interface, Flow_Table
    pub fn select_table(&mut self, table: &str) -> Result<Vec<u8>, std::io::Error> {
        let msg = format!(
            "{}{}{}{}{}{}{}",
            r#"{"method": "transact", "id": "#,
            self.msg_id,
            r#", "params": [""#,
            &self.db,
            r#"", {"op": "select", "table": ""#,
            table,
            r#"", "where": [] }]}"#
        );
        self.msg_id += 1;

        self.send_message(msg.as_ref())
    }

    pub fn create_bridge(&mut self, name: &str) -> Result<Vec<u8>, std::io::Error> {
        // Add a row to the Bridge table
        let msg = format!(
            "{}{}{}{}{}{}{}",
            r#"{"method": "transact", "id": "#,
            self.msg_id,
            r#", "params": [""#,
            &self.db,
            r#"", {"op": "insert", "table": "Bridge", "row": {"name": ""#,
            name,
            r#""}, "uuid-name": "my_new_bridge"}, {"op": "mutate", "table": "Open_vSwitch", "where": [], "mutations": [["bridges", "insert", ["named-uuid", "my_new_bridge"]]]}]}"#
        );
        self.msg_id += 1;
        println!("{}", msg);

        let response = self.send_message(msg.as_ref())?;
        Ok(response)

        /*
                println!("{}", String::from_utf8_lossy(&response));
                // Extract the UUID from the new bridge
                let response: OvsResult = serde_json::from_str(std::str::from_utf8(&response).unwrap())?;
                let uuid = &response.clone().result[0].clone().uuid.unwrap().1;
                println!("{:?}{}", response, uuid);

                // Tell Open vSwitch to manage this bridge
                let msg = format!(
                    "{}{}{}{}{}{}{}",
                    r#"{"method": "transact", "id": "#,
                    self.msg_id,
                    r#", "params": [""#,
                    &self.db,
                    r#"", {"op": "mutate", "table": "Open_vSwitch", "where": [], "mutations": [["bridges", "insert", ["uuid", ""#,
                    uuid,
                    r#""]]]}, {"op": "commit", "durable": true}]}"#
                );
                self.msg_id += 1;
                println!("{}", msg);

                self.send_message(msg.as_ref())
        */
    }

    pub fn create_port(&mut self, name: &str, bridge: &str) -> Result<Vec<u8>, std::io::Error> {
        let msg = format!(
            "{}{}{}{}{}{}{}",
            r#"{"method": "transact", "id": "#,
            self.msg_id,
            r#", "params": [""#,
            &self.db,
            r#"", {"op": "insert", "table": "Port", "row": {"name": ""#,
            name,
            r#"" }}]}"#
        );
        self.msg_id += 1;
        println!("{}", msg);

        self.send_message(msg.as_ref())
    }

    pub fn create_interface(&mut self, name: &str) -> Result<Vec<u8>, std::io::Error> {
        let msg = format!(
            "{}{}{}{}{}{}{}",
            r#"{"method": "transact", "id": "#,
            self.msg_id,
            r#", "params": [""#,
            &self.db,
            r#"", {"op": "insert", "table": "Interface", "row": {"name": ""#,
            name,
            r#"" }}, {"op": "commit", "durable": true}]}"#
        );
        self.msg_id += 1;
        println!("{}", msg);

        self.send_message(msg.as_ref())
        //        let response: OvsResult = serde_json::from_str(std::str::from_utf8(&response).unwrap())?;
        //        let uuid = &response.clone().result[0].unwrap().uuid.1;
        //        println!("{:?}\n{}", response, uuid);
        //
        //        let msg = format!(
        //            "{}{}{}{}{}{}{}{}{}",
        //            r#"{"method": "transact", "id": "#,
        //            self.msg_id,
        //            r#", "params": [""#,
        //            &self.db,
        //            r#"", {"op": "insert", "table": "Port", "row": {"name": ""#,
        //            name,
        //            r#"", "interfaces": [""#,
        //            uuid,
        //            r#""] }}]}"#
        //        );

        //        self.msg_id += 1;
        //        println!("{}", msg);
        //
        //        let response = self.send_message(msg.as_ref())?;
        //        println!("{}", String::from_utf8_lossy(&response));
        //
        //        let response: OvsResult = serde_json::from_str(std::str::from_utf8(&response).unwrap())?;
        //        let uuid = &response.clone().result[0].unwrap().uuid.1;
        //        println!("{:?}\n{}", response, uuid);
    }

    pub fn commit(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let msg = format!(
            "{}{}{}{}{}",
            r#"{"method": "transact", "id": "#,
            self.msg_id,
            r#", "params": [""#,
            &self.db,
            r#"", {"op": "commit", "durable": true}]}"#
        );
        self.msg_id += 1;
        println!("{}", msg);

        self.send_message(msg.as_ref())
    }
}

fn main() -> std::io::Result<()> {
    let mut ovs_sock = Ovs::new("/var/run/openvswitch/db.sock".into(), None)?;
    //    let response = ovs_sock.send_message(r#"{"method": "transact", "id": 0, "params": ["Open_vSwitch", {"op": "select", "table": "Bridge", "where": [] }]}"#)?;
    //    let response = ovs_sock.list_dbs()?;
    //    let response = ovs_sock.select_table("Interface")?;
    //    println!("{}", String::from_utf8(response).unwrap());

    let response = ovs_sock.select_table("Open_vSwitch")?;
    println!("{}", String::from_utf8(response).unwrap());

    let response = ovs_sock.select_table("Bridge")?;
    println!("{}", String::from_utf8(response).unwrap());

    let response = ovs_sock.create_bridge("test-br")?;
    println!("{}", String::from_utf8(response).unwrap());

    let response = ovs_sock.select_table("Bridge")?;
    println!("{}", String::from_utf8(response).unwrap());

    //    let response = ovs_sock.commit()?;
    //    println!("{}", String::from_utf8(response).unwrap());
    //

    Ok(())
}
