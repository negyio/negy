use anyhow::{bail, Result};

pub const PROTOCOL_SYMBOL_LEN: usize = 1;
pub const TUNNEL: u8 = 1;
pub const NODE_CONTEXT: u8 = 2;

#[derive(Copy, Clone)]
pub enum Protocol {
    Tunnel,
    NodeContext,
}

impl Protocol {
    pub fn symbol_byte(&self) -> u8 {
        match &self {
            Protocol::Tunnel => TUNNEL,
            Protocol::NodeContext => NODE_CONTEXT,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Protocol> {
        if bytes.len() == 0 {
            bail!("Protocol error. Symbol byte is 0 byte.")
        }

        let symbol_byte = bytes[0];

        match symbol_byte {
            TUNNEL => Ok(Protocol::Tunnel),
            NODE_CONTEXT => Ok(Protocol::NodeContext),
            _ => bail!("Protocol error. Unknown symbol byte {}", symbol_byte),
        }
    }
}
