use anyhow::Result;
use virtus::{config::Config, vm::*};

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

fn main() -> Result<()> {
    /*
     *
     * let conn = virtus::connect();
     * let config = virtus::Config::new();
     * let app = virtus::App::new(); // probably not this?
     *
     * let vm = virtus::VM::new(..., &conn);
     * let vm = virtus::VM::new(..., &conn, &config);
     * let vm = virtus::VM::new(..., &config);
     * let domain
     *
     *
     * let config = virtus::Config::new();
     * let conn = virtus::connect(&config);
     * or of course...
     * let conn = virtus::connect(virtus::Config::new());
     *
     * then
     *
     */

    let mut conn = virtus::connect(&Config::new())?;

    if let Ok(Some(vm)) = VM::find("new vm", &conn) {
        println!("{:?}", vm);
        vm.delete(&conn)?;
    }

    let mut network = Network::new(None, Some(0), Some("10.20.30.0/24".into()), &conn)
        .expect("failed to create network");

    let image = Image::new(
        String::from("/home/james/Downloads/ubuntu-22.04.3-live-server-amd64.iso"),
        true,
        &conn,
    )?;

    let mut domain = VM::new(
        "new vm",
        2,
        4 * virtus::GIGABYTE,
        20 * virtus::GIGABYTE,
        &image,
        &mut network,
        &conn,
    )?;

    println!("{}", domain.to_xml(&conn).unwrap());
    domain.build(&conn)?;

    conn.close()?;
    Ok(())
}
