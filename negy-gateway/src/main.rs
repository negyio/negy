#[macro_use]
extern crate log;

mod gateway;

use crate::gateway::{Gateway, NodeUnselected};
use anyhow::Result;
use clap::Parser;
use negy_node_pool::req::ListNodeResponse;
use openssl::rsa::Rsa;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, default_value = "0.0.0.0")]
    bind: String,
    #[clap(short, long, value_parser, default_value = "3000")]
    port: u16,
    #[clap(short, long, value_parser, default_value = "http://127.0.0.1:3030")]
    node_pool_endpoint: String,
    #[clap(short, long, value_parser, default_value = "3")]
    hops: usize,
    #[clap(short, long, value_parser, default_value = "0.1.5")]
    min_version: String,
}

async fn spawn_inner(
    client: TcpStream,
    node_pool: Arc<RwLock<Vec<NodeUnselected>>>,
    hops: usize,
) -> Result<()> {
    Gateway::new(client)
        .fetch_nodes(node_pool.clone(), hops)?
        .handshake()
        .await?
        .tunnel()
        .await?;

    Ok(())
}

async fn fetch_nodes_unselected(node_pool_endpoint: &str) -> Result<Vec<NodeUnselected>> {
    let res = reqwest::Client::new()
        .get(format!("{}/list", node_pool_endpoint))
        .send()
        .await?
        .json::<ListNodeResponse>()
        .await?;
    let args = Args::parse();
    let nodes_unselected: Vec<NodeUnselected> = res
        .nodes
        .into_iter()
        .map(|n| NodeUnselected {
            addr: n.addr,
            rsa: Rsa::public_key_from_pem(&base64::decode(&n.public_key).unwrap()).unwrap(),
            name: n.name,
            version: n.version,
        })
        .filter(|n| n.version < args.min_version)
        .collect();

    Ok(nodes_unselected)
}

async fn spawn(listener: TcpListener, node_pool_endpoint: String, hops: usize) -> Result<()> {
    let listed_nodes: Arc<RwLock<Vec<NodeUnselected>>> = Arc::new(RwLock::new(Vec::new()));
    let listed_nodes_fetch = listed_nodes.clone();
    let listed_nodes_accept = listed_nodes.clone();

    tokio::spawn(async move {
        loop {
            match fetch_nodes_unselected(&node_pool_endpoint).await {
                Ok(nodes_unselected) => {
                    info!("fetched {} nodes", nodes_unselected.len());
                    *listed_nodes_fetch.write().unwrap() = nodes_unselected;
                }
                Err(e) => {
                    warn!("failed to fetch nodes from node pool. it seems node pool is down.");
                    warn!("node list was not renewed.");
                    debug!("{:?}", e);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    loop {
        let (client, _) = listener.accept().await?;
        let listed_nodes = listed_nodes_accept.clone();

        tokio::spawn(async move {
            if let Err(e) = spawn_inner(client, listed_nodes, hops).await {
                error!("{:?}", e);
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();

    ctrlc::set_handler(move || {
        warn!("receive terminate signal... process will be exited.");
        std::process::exit(1);
    })?;

    let args = Args::parse();
    let bind_addr = format!("{}:{}", args.bind, args.port);

    info!("start listening on {}", bind_addr);

    let listener = TcpListener::bind(bind_addr).await?;

    spawn(listener, args.node_pool_endpoint, args.hops).await?;

    Ok(())
}
