use virtus::ovs;

fn main() {
    let mut ovs_conn = ovs::Ovs::new().connect().expect("failed to connect");
    let mut bridge = ovs::bridge::Bridge::new(String::from("test-br-7"));

    //    let bridges = ovs_conn.get_bridges().unwrap();
    //    println!("{:?}", bridges);

    let virtus_int = ovs_conn.find_bridge(String::from("virtus-int")).unwrap();
    //   println!("{:?}", virtus_int);

    let ports = ovs_conn.get_ports().unwrap();
    //    println!("{:?}", ports);

    let mut new_port = ovs::port::Port::new(String::from("test-port"), String::from("virtus-int"));
    let new_id = ovs_conn
        .create(&mut new_port)
        .expect("failed to create port");



    ovs_conn.delete(new_port).expect("failed to delete bridge");

    /*
    let new_id = ovs_conn.create(&mut bridge).expect("failed to create bridge");
    println!("{:?}, {:?}", bridge, new_id);

    ovs_conn.delete(bridge).expect("failed to delete bridge");


    let ports = ovs_conn.get_ports().unwrap();
    println!("{:?}", ports);

    let interfaces = ovs_conn.get_interfaces().unwrap();
    println!("{:?}", interfaces);
    */
}
