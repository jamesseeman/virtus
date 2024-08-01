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

    #[test]
    pub fn create_qcow() {
        let new_disk = virtus::vm::Disk::create(1024).unwrap();
        println!("{:?}", new_disk);
        let filename = new_disk.delete().unwrap();
        println!("{}", filename);
    }

    #[test]
    fn create_network() {
        println!("todo");
    }
}

fn main() {
    let uri = "qemu:///system";

    let mut conn =
        virt::connect::Connect::open(uri).expect(format!("Failed to connect to {}", uri).as_str());

    let new_vm = virtus::vm::VM::find(String::from("new vm"), &conn);
    println!("{:?}", new_vm);

    if let Ok(Some(vm)) = new_vm {
        vm.delete().unwrap();
    }

    /*
    let network = virtus::vm::Network::new(Some(0), None, true, Some("10.20.30.0/24".into()))
        .expect("failed to create network");

    let image = virtus::vm::Image::new(String::from("/home/james/Downloads/ubuntu-22.04.3-live-server-amd64.iso"), false);

    let disk = virtus::vm::Disk::create(10 * 1024 * 1024 * 1024).expect("failed to provision disk");
    let mut domain = virtus::vm::VM::new(String::from("new vm"), 2, 4096, disk, image, network);

    println!("{}", domain.to_xml().unwrap());
    domain.build(&conn).unwrap();
    */

    conn.close().unwrap();
}
