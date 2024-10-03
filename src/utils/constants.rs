use alloy::primitives::{address, b256, Address, FixedBytes};

pub const INIT_CODE_HASH: FixedBytes<32> =
    b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");
pub const PROXY_FACTORY_ADDRESS: Address = address!("aacFeEa03eb1561C4e67d661e40682Bd20E3541b");
pub const OUT_FILE_PATH: &str = "data/out.txt";
