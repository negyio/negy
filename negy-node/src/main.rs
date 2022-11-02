#[macro_use]
extern crate log;

mod node;

use crate::node::Node;
use anyhow::{bail, Result};
use clap::Parser;
use negy_common::protocol::Protocol;
use negy_node_pool::req::AddNodeRequest;
use openssl::{pkey::Private, rsa::Rsa};
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
}

async fn spawn_inner(client: TcpStream, rsa: Rsa<Private>) -> Result<()> {
    let node = Node::new(client, rsa).accept().await?;

    match node.protocol() {
        Protocol::Tunnel => node.handshake().await?.tunnel().await?,
        Protocol::NodeContext => node.serve_context().await?,
    }

    Ok(())
}

async fn add_request(rsa: &Rsa<Private>, port: u16, node_pool_endpoint: &str) -> Result<()> {
    info!("send add/update request to node pool");

    let version: &str = env!("CARGO_PKG_VERSION");

    let req = AddNodeRequest {
        port,
        public_key: base64::encode(rsa.public_key_to_pem().unwrap()),
        version: version.to_owned(),
    };
    let res = reqwest::Client::new()
        .post(format!("{}/add", node_pool_endpoint))
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await?;

    if res.status() != reqwest::StatusCode::OK {
        error!("{:?}", res);
        error!("{:?}", res.text().await?);
        bail!("failed to add this node to node pool")
    }

    info!("successfully add/update the node information on node pool!");

    Ok(())
}

async fn connect_to_node_pool(
    rsa: Rsa<Private>,
    port: u16,
    node_pool_endpoint: String,
) -> Result<()> {
    loop {
        if let Err(e) = add_request(&rsa, port, &node_pool_endpoint).await {
            error!("failed to add this node to node pool");
            error!("{:?}", e);
        }

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn spawn(listener: TcpListener, port: u16, node_pool_endpoint: String) -> Result<()> {
    let rsa = Rsa::generate(2048)?;
    let rsa_node_pool_connection = rsa.clone();

    tokio::spawn(async move {
        if let Err(e) =
            connect_to_node_pool(rsa_node_pool_connection, port, node_pool_endpoint).await
        {
            error!("{:?}", e);
        }
    });

    loop {
        let (client, _) = listener.accept().await?;
        let rsa = rsa.clone();

        tokio::spawn(async move {
            if let Err(e) = spawn_inner(client, rsa).await {
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

    spawn(listener, args.port, args.node_pool_endpoint).await?;

    Ok(())
}
