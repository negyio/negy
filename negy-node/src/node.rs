use anyhow::{anyhow, bail, Result};
use bytes::BytesMut;
use negy_common::aes::Aes;
use negy_common::encrypted_payload::{EncryptedPayload, DELIMITER_LEN};
use negy_common::protocol::{Protocol, PROTOCOL_SYMBOL_LEN};
use openssl::pkey::Private;
use openssl::rsa::{Padding, Rsa};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct StateInit {
    rsa: Rsa<Private>,
    client: TcpStream,
}

pub struct StateAccepted {
    rsa: Rsa<Private>,
    protocol: Protocol,
    client: TcpStream,
    payload_init: BytesMut,
}

pub struct StateTunnel {
    aes: Aes,
    delimiter: [u8; DELIMITER_LEN],
    client: TcpStream,
    upstream: TcpStream,
}

pub struct Node<State> {
    state: State,
}

impl Node<StateInit> {
    pub fn new(client: TcpStream, rsa: Rsa<Private>) -> Self {
        Node {
            state: StateInit { rsa, client },
        }
    }

    pub async fn accept(mut self) -> Result<Node<StateAccepted>> {
        let mut c_bytes = [0; 4096];

        let (mut c_rx, _) = self.state.client.split();

        let n = c_rx.read(&mut c_bytes).await?;
        let payload_init = BytesMut::from(&c_bytes[..n]);
        let protocol = Protocol::parse(&payload_init)?;

        Ok(Node {
            state: StateAccepted {
                protocol,
                rsa: self.state.rsa,
                client: self.state.client,
                payload_init,
            },
        })
    }
}

impl Node<StateAccepted> {
    pub fn protocol(&self) -> Protocol {
        self.state.protocol
    }

    pub async fn serve_context(mut self) -> Result<()> {
        let (_, mut c_tx) = self.state.client.split();
        let version: &str = env!("CARGO_PKG_VERSION");

        c_tx.write_all(&self.state.rsa.public_key_to_pem()?).await?;
        c_tx.write_all(version.as_bytes()).await?;

        Ok(())
    }

    pub async fn handshake(mut self) -> Result<Node<StateTunnel>> {
        let mut u_bytes = [0; 4096];

        let (_, mut c_tx) = self.state.client.split();

        let payload_len: usize = PROTOCOL_SYMBOL_LEN + self.state.rsa.size() as usize * 3;
        let payload_self = &self.state.payload_init[..payload_len];
        let payload_successor = &self.state.payload_init[payload_len..];

        let rsa_key_len: usize = self.state.rsa.size() as usize;
        let mut decrypted_delimiter = vec![0; rsa_key_len];
        self.state.rsa.private_decrypt(
            &payload_self[PROTOCOL_SYMBOL_LEN..PROTOCOL_SYMBOL_LEN + rsa_key_len],
            &mut decrypted_delimiter,
            Padding::PKCS1,
        )?;

        let mut delimiter = [0; DELIMITER_LEN];
        delimiter.copy_from_slice(&decrypted_delimiter[..DELIMITER_LEN]);

        let mut decrypted_dist = vec![0; rsa_key_len];
        self.state.rsa.private_decrypt(
            &payload_self[PROTOCOL_SYMBOL_LEN + rsa_key_len..PROTOCOL_SYMBOL_LEN + rsa_key_len * 2],
            &mut decrypted_dist,
            Padding::PKCS1,
        )?;

        let mut decrypted_aes = vec![0; rsa_key_len];
        self.state.rsa.private_decrypt(
            &payload_self
                [PROTOCOL_SYMBOL_LEN + rsa_key_len * 2..PROTOCOL_SYMBOL_LEN + rsa_key_len * 3],
            &mut decrypted_aes,
            Padding::PKCS1,
        )?;

        let mut key_iv = [0; 48];
        key_iv.copy_from_slice(&decrypted_aes[..48]);
        let aes = Aes::import(&key_iv);

        let dist = std::str::from_utf8(
            &decrypted_dist
                .splitn(2, |b| b == &b'\0')
                .next()
                .ok_or(anyhow!("failed to find dist in payload"))?,
        )?;

        let mut upstream = TcpStream::connect(dist).await?;
        let (mut u_rx, mut u_tx) = upstream.split();

        if payload_successor.len() > 0 {
            u_tx.write_all(&payload_successor).await?;

            let n = u_rx.read(&mut u_bytes).await?;

            if n != 2 && u_bytes[..2] != b"OK"[..] {
                bail!("invalid response by upstream")
            }
        }

        c_tx.write_all("OK".as_bytes()).await?;

        Ok(Node {
            state: StateTunnel {
                aes,
                delimiter,
                client: self.state.client,
                upstream,
            },
        })
    }
}

impl Node<StateTunnel> {
    pub async fn tunnel(&mut self) -> Result<()> {
        let mut c_bytes = [0; 4096];
        let mut u_bytes = [0; 4096];

        let (mut c_rx, mut c_tx) = self.state.client.split();
        let (mut u_rx, mut u_tx) = self.state.upstream.split();

        let mut encrypted_payload = EncryptedPayload::new();

        loop {
            tokio::select! {
                n = c_rx.read(&mut c_bytes) => {
                    match n {
                        Ok(0) => break,
                        Ok(n) => {
                            let payloads = encrypted_payload.read(&c_bytes[..n], &self.state.delimiter)?;

                            for payload in payloads {
                                let decrypted = self.state.aes.decrypt(&payload)?;
                                u_tx.write_all(&decrypted).await?;
                            }
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
                            let encrypted = self.state.aes.encrypt(&u_bytes[..n])?;
                            let mut payload = BytesMut::from(&encrypted as &[u8]);
                            payload.extend_from_slice(&self.state.delimiter);

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
