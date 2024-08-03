use anyhow::Result;
use futures::stream::TryStreamExt;
use netlink_packet_route::link::LinkMessage;

#[tokio::main]
pub async fn main() -> Result<()> {
    let (connection, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(connection);

    let mut links = handle
        .link()
        .get()
        .match_name(String::from("vlan.20"))
        .execute()
        .try_collect::<Vec<_>>()
        .await?;
        
    if let Some(link) = links.pop() {
        let addrs = handle.address().get().set_link_index_filter(link.header.index).execute().try_collect::<Vec<_>>().await?;
        println!("{:?}", addrs);
    }

    Ok(())
}
