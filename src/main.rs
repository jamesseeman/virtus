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

    let network = virtus::vm::Network::new(Some(0), None, true, Some("10.20.30.0/24".into()));

    conn.close().unwrap();
}
