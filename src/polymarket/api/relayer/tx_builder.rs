use std::sync::Arc;

use alloy::{
    dyn_abi::SolType,
    primitives::{Address, Bytes, U256},
    signers::{Signature, Signer},
    sol,
    sol_types::{eip712_domain, SolCall, SolStruct, SolValue},
};

use crate::utils::poly::get_proxy_wallet_address;

sol! {
    function multiSend(bytes bytes) external payable;

    #[derive(Debug)]
    struct SafeTx {
        address to;
        uint256 value;
        bytes data;
        uint8 operation;
        uint256 safeTxGas;
        uint256 baseGas;
        uint256 gasPrice;
        address gasToken;
        address refundReceiver;
        uint256 nonce;
    }

    #[derive(Debug)]
    struct RelayerTransaction {
        uint8 operation;
        address to;
        uint256 value;
        uint256 dataLen;
        bytes data;
    }
}

impl SafeTx {
    fn new(data: Bytes, operation: u8, nonce: U256, to: Address) -> Self {
        Self {
            to,
            value: U256::ZERO,
            data,
            operation,
            safeTxGas: U256::ZERO,
            baseGas: U256::ZERO,
            gasPrice: U256::ZERO,
            gasToken: Address::ZERO,
            refundReceiver: Address::ZERO,
            nonce,
        }
    }
}

impl RelayerTransaction {
    pub fn new(operation: u8, to: Address, value: U256, data: Bytes) -> Self {
        let data_len = data.len();

        Self {
            operation,
            to,
            value,
            dataLen: U256::from(data_len),
            data,
        }
    }
}

pub fn get_multisend_calldata(transactions: Vec<RelayerTransaction>) -> Vec<u8> {
    let bytes = transactions
        .into_iter()
        .flat_map(|tx| tx.abi_encode_packed())
        .collect::<Vec<u8>>();

    multiSendCall {
        bytes: bytes.into(),
    }
    .abi_encode()
}

pub async fn get_packed_signature<S>(
    signer: Arc<S>,
    operation: u8,
    nonce: U256,
    encoded_call: Vec<u8>,
    to: Address,
) -> eyre::Result<String>
where
    S: Signer + Sync + Send,
{
    let proxy_wallet_address = get_proxy_wallet_address(signer.clone());

    let domain = eip712_domain! {
        chain_id: 137,
        verifying_contract: proxy_wallet_address,
    };

    let transaction = SafeTx::new(encoded_call.into(), operation, nonce, to);
    let message = transaction.eip712_signing_hash(&domain);

    let signature = signer.sign_message(message.as_slice()).await?;
    let parity = get_v_incremented(&signature);

    let packed_signature = <sol! {(uint256, uint256, uint8)}>::abi_encode_packed(&(
        &signature.r(),
        &signature.s(),
        &parity,
    ));

    Ok(const_hex::encode_prefixed(packed_signature))
}

fn get_v_incremented(signature: &Signature) -> u8 {
    let mut v = signature.v().to_u64();

    match v {
        0 | 1 => v += 31,
        27 | 28 => v += 4,
        _ => unreachable!(),
    }

    v as u8
}
