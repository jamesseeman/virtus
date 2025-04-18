use anyhow::Result;
use futures::stream::TryStreamExt;
use virtus::*;

#[cfg(test)]
mod tests {
    const URI: &str = "qemu:///system";

    fn connect_uri(uri: &str) -> Result<virt::connect::Connect, virt::error::Error> {
        virt::connect::Connect::open(uri)
    }

    #[test]
    fn connect() {
        let mut conn = connect_uri(URI).expect("failed to connect");

        conn.close().unwrap();
    }

    /*
    #[test]
    pub fn create_qcow() {
        let new_disk = virtus::vm::Disk::create(1024).unwrap();
        println!("{:?}", new_disk);
        let filename = new_disk.delete().unwrap();
        println!("{}", filename);
    }
    */

    #[test]
    fn create_network() {
        println!("todo");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut conn = virtus::connect(&Config::new()).await?;

    if let Ok(Some(vm)) = VM::find("new vm 2", &conn) {
        println!("{:?}", vm);
        vm.delete(&conn).await?;
    }

    /*
    println!("Disks: {:?}", Disk::list(&conn)?);
    println!("Images: {:?}", Image::list(&conn)?);
    println!("Networks: {:?}", Network::list(&conn)?);
    println!("Interfaces: {:?}", Interface::list(&conn)?);
    println!("VMs: {:?}", VM::list(&conn)?);

    for disk in Disk::list(&conn)? {
        Disk::delete_by_id(disk, &conn)?;
    }

    for image in Image::list(&conn)? {
        Image::delete_by_id(image, &conn)?;
    }

    for network in Network::list(&conn)? {
        Network::delete_by_id(network, &conn)?;
    }

    for interface in Interface::list(&conn)? {
        Interface::delete_by_id(interface, &conn)?;
    }

    for vm in VM::list(&conn)? {
        VM::delete_by_id(vm, &conn)?;
    }
    */

    /*
    let mut network = match Network::get_network_by_uplink("vlan.20", &conn)? {
        Some(found) => found,
        None => Network::new(
            "test network",
            Some(0),
            Some("10.20.30.0/24".into()),
            Some("vlan.20"),
            &conn,
        )
        .await
        .expect("failed to create network"),
    };
    println!("{:?}", network);
    */

    //    let mut network = match Network::find("t1", &conn)? {
    //        Some(network) => network,
    //        None => Network::new("t1", Some(0), Some("10.20.30.0/24"), None, &conn).await?,
    //    };

    let mut network = match Network::find("host", &conn)? {
        Some(network) => network,
        None => Network::new("host", None, None, Some("vlan.20"), &conn).await?,
    };

    let image = Image::new(
        String::from("/home/james/Downloads/ubuntu-22.04.3-live-server-amd64.iso"),
        true,
        &conn,
    )?;

    let mut domain = VM::new(
        "new vm 2",
        2,
        4 * virtus::GIGABYTE,
        20 * virtus::GIGABYTE,
        &image,
        &mut network,
        &conn,
    )
    .await?;

    println!("{}", domain.to_xml(&conn).await?);
    domain.build(&conn).await?;

    conn.close()?;
    Ok(())
}
