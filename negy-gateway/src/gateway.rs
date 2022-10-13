use anyhow::{bail, Result};
use bytes::{BufMut, BytesMut};
use negy_common::aes::Aes;
use negy_common::encrypted_payload::{EncryptedPayload, DELIMITER_LEN};
use negy_common::protocol::Protocol;
use openssl::pkey::Public;
use openssl::rsa::{Padding, Rsa};
use rand::seq::SliceRandom;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct StateFetchNodes {
    client: TcpStream,
}

pub struct StateHandshake {
    client: TcpStream,
    nodes: Vec<Node>,
}

pub struct StateTunnel {
    client: TcpStream,
    upstream: TcpStream,
    nodes: Vec<Node>,
}

pub struct NodeUnselected {
    pub addr: SocketAddr,
    pub rsa: Rsa<Public>,
}

struct Node {
    aes: Aes,
    rsa: Rsa<Public>,
    delimiter: [u8; DELIMITER_LEN],
    dist: SocketAddr,
    encrypted_payload: EncryptedPayload,
}

pub struct Gateway<State> {
    state: State,
}

impl Gateway<StateFetchNodes> {
    pub fn new(client: TcpStream) -> Self {
        Gateway {
            state: StateFetchNodes { client },
        }
    }

    pub fn fetch_nodes(
        self,
        nodes: Arc<RwLock<Vec<NodeUnselected>>>,
        hops: usize,
    ) -> Result<Gateway<StateHandshake>> {
        let mut rng = &mut rand::thread_rng();
        let random_selected_nodes: Vec<Node> = nodes
            .read()
            .unwrap()
            .choose_multiple(&mut rng, hops)
            .into_iter()
            .map(|n| Node {
                aes: Aes::new(),
                rsa: n.rsa.clone(),
                delimiter: EncryptedPayload::new_delimiter(),
                dist: n.addr,
                encrypted_payload: EncryptedPayload::new(),
            })
            .collect();

        if random_selected_nodes.len() < hops {
            bail!(
                "not enough nodes (hops={}, nodes={})",
                hops,
                random_selected_nodes.len()
            )
        }

        Ok(Gateway {
            state: StateHandshake {
                client: self.state.client,
                nodes: random_selected_nodes,
            },
        })
    }
}

impl Gateway<StateHandshake> {
    pub async fn handshake(mut self) -> Result<Gateway<StateTunnel>> {
        let addrs = self.parse_http().await?;

        let mut u_bytes = [0; 4096];
        let mut upstream = TcpStream::connect(self.state.nodes.first().unwrap().dist).await?;
        let (mut u_rx, mut u_tx) = upstream.split();

        let mut payload = BytesMut::new();
        let mut dist = addrs[0];

        for n in self.state.nodes.iter().rev() {
            let mut encrypted_delimiter = vec![0; n.rsa.size() as usize];
            n.rsa
                .public_encrypt(&n.delimiter, &mut encrypted_delimiter, Padding::PKCS1)?;

            let mut encrypted_dist = vec![0; n.rsa.size() as usize];
            n.rsa.public_encrypt(
                dist.to_string().as_bytes(),
                &mut encrypted_dist,
                Padding::PKCS1,
            )?;

            let mut encrypted_aes = vec![0; n.rsa.size() as usize];
            n.rsa
                .public_encrypt(&n.aes.get_key_iv(), &mut encrypted_aes, Padding::PKCS1)?;

            let mut bytes = BytesMut::new();
            bytes.put_u8(Protocol::Tunnel.symbol_byte());
            bytes.extend_from_slice(&encrypted_delimiter);
            bytes.extend_from_slice(&encrypted_dist);
            bytes.extend_from_slice(&encrypted_aes);

            let mut tmp = BytesMut::new();
            tmp.extend_from_slice(&bytes);
            tmp.extend_from_slice(&payload);
            payload = tmp;
            dist = n.dist;
        }

        u_tx.write_all(&payload).await?;

        let n = u_rx.read(&mut u_bytes).await?;

        if n != 2 && u_bytes[..2] != b"OK"[..] {
            bail!("invalid response by upstream")
        }

        self.response_200().await?;

        Ok(Gateway {
            state: StateTunnel {
                client: self.state.client,
                upstream,
                nodes: self.state.nodes,
            },
        })
    }

    async fn parse_http(&mut self) -> Result<Vec<SocketAddr>> {
        let mut c_bytes = [0; 4096];
        let (mut c_rx, _) = self.state.client.split();
        let n = c_rx.read(&mut c_bytes).await?;

        let req_raw = std::str::from_utf8(&c_bytes[..n])?;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);

        req.parse(req_raw.as_bytes())?;

        match req.method {
            Some("CONNECT") => {}
            Some(method) => bail!("unsupported method {}. Only CONNECT is supported.", method),
            None => bail!("HTTP method not found in your request."),
        }

        Ok(req.path.unwrap().to_socket_addrs().unwrap().collect())
    }

    async fn response_200(&mut self) -> Result<()> {
        let (_, mut c_tx) = self.state.client.split();

        c_tx.write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).await?;

        Ok(())
    }
}

impl Gateway<StateTunnel> {
    pub async fn tunnel(&mut self) -> Result<()> {
        let mut c_bytes = [0; 4096];
        let mut u_bytes = [0; 4096];

        let (mut c_rx, mut c_tx) = self.state.client.split();
        let (mut u_rx, mut u_tx) = self.state.upstream.split();

        loop {
            tokio::select! {
                n = c_rx.read(&mut c_bytes) => {
                    match n {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut payload = BytesMut::from(&c_bytes[..n]);

                            for node in self.state.nodes.iter().rev() {
                                let encrypted = node.aes.encrypt(&payload)?;
                                let mut tmp = BytesMut::from(&encrypted as &[u8]);
                                tmp.extend_from_slice(&node.delimiter);
                                payload = tmp;
                            }

                            u_tx.write_all(&payload).await?;
                        },
                        Err(e) => {
                            error!("client read {:?}", e);
                        }
                    }
                }
                n = u_rx.read(&mut u_bytes) => {
                    match n {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut payload = BytesMut::from(&u_bytes[..n]);

                            for node in self.state.nodes.iter_mut() {
                                let payloads = node.encrypted_payload.read(&payload, &node.delimiter)?;
                                let mut tmp = BytesMut::new();

                                for p in payloads {
                                    match node.aes.decrypt(&p) {
                                        Ok(decrypted) => {
                                            tmp.extend_from_slice(&decrypted)
                                        },
                                        Err(e) => {
                                            return Err(e);
                                        },
                                    }
                                }

                                payload = tmp;
                            }

                            c_tx.write_all(&payload).await?;
                        },
                        Err(e) => {
                            error!("upstream read {:?}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
