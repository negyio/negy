#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use clap::Parser;
use negy_common::protocol::Protocol;
use negy_node_pool::req::{AddNodeRequest, ListNodeResponse, ListedNode};
use openssl::rsa::Rsa;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use warp::Filter;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, default_value = "0.0.0.0")]
    bind: String,
    #[clap(short, long, value_parser, default_value = "3030")]
    port: u16,
}

#[derive(Debug, Clone)]
struct Node {
    public_key: String,
    version: String,
}

#[derive(Debug)]
struct InvalidParameters;

impl warp::reject::Reject for InvalidParameters {}

async fn list(
    node_pool: Arc<RwLock<HashMap<SocketAddr, Node>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let nodes = node_pool.read().unwrap().clone();
    let nodes = nodes
        .into_iter()
        .map(|(addr, node)| ListedNode {
            addr,
            public_key: node.public_key,
            version: node.version,
        })
        .collect();

    Ok(warp::reply::json(&ListNodeResponse { nodes }))
}

async fn add(
    node_pool: Arc<RwLock<HashMap<SocketAddr, Node>>>,
    addr_cloud_front: Option<SocketAddr>,
    addr: Option<SocketAddr>,
    body: AddNodeRequest,
) -> Result<impl warp::Reply, warp::Rejection> {
    let addr = if let Some(addr) = addr_cloud_front {
        addr
    } else if let Some(addr) = addr {
        addr
    } else {
        warn!("failed to resolve node addr");
        return Err(warp::reject::custom(InvalidParameters));
    };

    let addr = SocketAddr::new(addr.ip(), body.port);

    info!("new add request {}", addr);

    // validate base64 & RSA public key
    let public_key_bytes =
        base64::decode(&body.public_key).map_err(|_| warp::reject::custom(InvalidParameters))?;

    Rsa::public_key_from_pem(&public_key_bytes)
        .map_err(|_| warp::reject::custom(InvalidParameters))?;

    if healthcheck_node(&addr, &body.public_key, &body.version)
        .await
        .is_ok()
    {
        node_pool.write().unwrap().insert(
            addr,
            Node {
                public_key: body.public_key,
                version: body.version,
            },
        );
        info!("new node has been added {}", addr);
    } else {
        warn!(
            "cannot connect to the node. may be it's not public ip {}",
            addr
        );
    }

    Ok(warp::reply::with_status("ok", warp::http::StatusCode::OK))
}

async fn pong() -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::with_status("ok", warp::http::StatusCode::OK))
}

async fn healthcheck_node(addr: &SocketAddr, public_key: &str, version: &str) -> Result<()> {
    let mut node = TcpStream::connect(addr).await?;
    let mut bytes = [0; 1024];
    let (mut rx, mut tx) = node.split();

    tx.write_u8(Protocol::NodeContext.symbol_byte()).await?;

    let n = rx.read(&mut bytes).await?;
    let public_key_received = base64::encode(&bytes[..451]);
    let version_received = std::str::from_utf8(&bytes[451..n])?;

    if public_key_received != public_key {
        bail!("public key mismatch")
    }

    if version != version_received {
        bail!("version mismatch")
    }

    Ok(())
}

async fn healthcheck_loop(node_pool: Arc<RwLock<HashMap<SocketAddr, Node>>>) -> Result<()> {
    loop {
        let mut removed_count = 0;
        let node_pool_for_iter = node_pool.read().unwrap().clone();

        for (addr, node) in node_pool_for_iter.iter() {
            if let Err(_) = healthcheck_node(addr, &node.public_key, &node.version).await {
                node_pool.write().unwrap().remove(addr);
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!("removed {} nodes", removed_count);
        }

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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

    let node_pool: Arc<RwLock<HashMap<SocketAddr, Node>>> = Arc::new(RwLock::new(HashMap::new()));
    let node_pool_healthcheck = node_pool.clone();
    let node_pool_filter = warp::any().map(move || node_pool.clone());

    let args = Args::parse();
    let bind_addr = format!("{}:{}", args.bind, args.port);

    let add = warp::path!("add")
        .and(warp::filters::method::post())
        .and(node_pool_filter.clone())
        .and(warp::filters::header::optional("CloudFront-Viewer-Address"))
        .and(warp::addr::remote())
        .and(warp::filters::body::json::<AddNodeRequest>())
        .and_then(add);

    let list = warp::path!("list")
        .and(warp::filters::method::get())
        .and(node_pool_filter.clone())
        .and_then(list);

    let pong = warp::path!("ping")
        .and(warp::filters::method::get())
        .and_then(pong);

    tokio::spawn(async move {
        if let Err(e) = healthcheck_loop(node_pool_healthcheck).await {
            error!("{:?}", e);
        }
    });

    warp::serve(add.or(list).or(pong))
        .run(bind_addr.to_socket_addrs().unwrap().next().unwrap())
        .await;

    Ok(())
}
