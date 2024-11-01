use anyhow::Result;
use futures::stream::TryStreamExt;
use netlink_packet_route::link::LinkMessage;
use rtnetlink::Handle;
use std::{thread, time};

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
        let addrs = handle
            .address()
            .get()
            .set_link_index_filter(link.header.index)
            .execute()
            .try_collect::<Vec<_>>()
            .await?;
        println!("{:?}", addrs);
    }

    handle
        .link()
        .add()
        .bridge(String::from("virtus-int"))
        .execute()
        .await?;

    /*
     if let Some(link) = get_link("test-br", &handle).await? {
         println!("{}", link.header.index);
    //     handle.link().set(link.header.index).master
         handle.link().del(link.header.index).execute().await?;
     }
     */

    Ok(())
}

pub async fn get_link(name: &str, handle: &Handle) -> Result<Option<LinkMessage>> {
    let mut links = handle
        .link()
        .get()
        .match_name(name.into())
        .execute()
        .try_collect::<Vec<_>>()
        .await?;
    Ok(links.pop())
}
