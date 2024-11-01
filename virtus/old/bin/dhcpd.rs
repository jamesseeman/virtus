use tokio::net::UdpSocket;
//use bytes::{Buf, BufMut, BytesMut};
use anyhow::Result;
use dhcproto::v4::{self, Decodable};
use pnet::datalink;
use std::collections::HashMap;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    //    let config = Config::new()?;
    //    let vm_tree = config.db.open_tree("vms")?;
    //    vm_tree.insert("hm", ":(");

    //let interfaces = datalink::interfaces();
    //let iface: Vec<datalink::NetworkInterface> = interfaces.into_iter().filter(|iface| iface.name == String::from("virtus-int")).collect();
    //println!("{:?}", iface);

    let socket = UdpSocket::bind("0.0.0.0:67").await?;
    //socket.bind_device(Some(b"virtus-int"))?;

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        println!("{}, {}", len, addr);

        let msg = v4::Message::decode(&mut v4::Decoder::new(&buf)).unwrap();
        println!("{:?}, {:?}", msg, msg.chaddr());

        let opts = msg.opts().get(v4::OptionCode::MessageType);
        println!("{:?}", opts);

        //        let response = v4::Message::default();
        //        response.set_flags(dhcproto::v4::Flags::
    }

    Ok(())
}
