use std::sync::Arc;

use alloy::{
    dyn_abi::SolType,
    network::Ethereum,
    primitives::{address, Address, U256},
    providers::Provider,
    sol,
    sol_types::{SolCall, SolValue},
    transports::Transport,
};
use itertools::Itertools;

use crate::{
    onchain::constants::EXPECTED_CHECK_APPROVALS_RESULT,
    polymarket::api::relayer::constants::{
        CONDITIONAL_TOKENS_CONTRACT_ADDRESS, UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
    },
};

use super::{
    client::IERC20::{allowanceCall, balanceOfCall},
    constants::MULTICALL_CONTRACT_ADDRESS,
    types::token::Token,
};

sol! {
    #[derive(Debug)]
    struct Result {
        bool success;
        bytes returnData;
    }

    #[derive(Debug)]
    struct Call3 {
        address target;
        bool allowFailure;
        bytes callData;
    }

    #[sol(rpc)]
    contract Multicall3 {
        function aggregate3(Call3[] calldata calls) public payable returns (Result[] memory returnData);
    }

    function isApprovedForAll(address owner, address operator) external view returns (bool);
}

const CTF_EXCHANGE_CONTRACT_ADDRESS: Address = address!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");
const NEG_RISK_CTF_EXCHANGE_CONTRACT_ADDRESS: Address =
    address!("C5d563A36AE78145C45a50134d48A1215220f80a");
const NEG_RISK_ADAPTER_CONTRACT_ADDRESS: Address =
    address!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");

pub async fn multicall_balance_of<P, T>(
    addresses: &[Address],
    token: Token,
    provider: Arc<P>,
) -> eyre::Result<Vec<U256>>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let calls = addresses
        .iter()
        .map(|address| {
            let calldata = balanceOfCall::new((*address,)).abi_encode();
            Call3 {
                target: token.contract_address,
                allowFailure: false,
                callData: calldata.into(),
            }
        })
        .collect_vec();

    let multicall_instance = Multicall3::new(MULTICALL_CONTRACT_ADDRESS, provider);
    let result = multicall_instance
        .aggregate3(calls)
        .call()
        .await?
        .returnData
        .iter()
        .map(|balance| {
            <sol! { uint256 }>::abi_decode(&balance.returnData, false).unwrap_or(U256::ZERO)
        })
        .collect::<Vec<_>>();

    Ok(result)
}

pub async fn check_token_approvals<P, T>(
    provider: Arc<P>,
    wallet_address: Address,
) -> eyre::Result<bool>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let multicall_instance = Multicall3::new(MULTICALL_CONTRACT_ADDRESS, provider);

    let calls = vec![
        (
            UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
            allowanceCall::new((wallet_address, CONDITIONAL_TOKENS_CONTRACT_ADDRESS)).abi_encode(),
        ),
        (
            UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
            allowanceCall::new((wallet_address, CTF_EXCHANGE_CONTRACT_ADDRESS)).abi_encode(),
        ),
        (
            CONDITIONAL_TOKENS_CONTRACT_ADDRESS,
            isApprovedForAllCall::new((wallet_address, CTF_EXCHANGE_CONTRACT_ADDRESS)).abi_encode(),
        ),
        (
            UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
            allowanceCall::new((wallet_address, NEG_RISK_CTF_EXCHANGE_CONTRACT_ADDRESS))
                .abi_encode(),
        ),
        (
            UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
            allowanceCall::new((wallet_address, NEG_RISK_ADAPTER_CONTRACT_ADDRESS)).abi_encode(),
        ),
        (
            CONDITIONAL_TOKENS_CONTRACT_ADDRESS,
            isApprovedForAllCall::new((wallet_address, NEG_RISK_CTF_EXCHANGE_CONTRACT_ADDRESS))
                .abi_encode(),
        ),
        (
            CONDITIONAL_TOKENS_CONTRACT_ADDRESS,
            isApprovedForAllCall::new((wallet_address, NEG_RISK_ADAPTER_CONTRACT_ADDRESS))
                .abi_encode(),
        ),
    ]
    .into_iter()
    .map(|(address, calldata)| Call3 {
        target: address,
        allowFailure: false,
        callData: calldata.into(),
    })
    .collect_vec();

    let result = multicall_instance
        .aggregate3(calls)
        .call()
        .await?
        .returnData
        .into_iter()
        .flat_map(|res| res.abi_encode())
        .collect_vec();

    match result != EXPECTED_CHECK_APPROVALS_RESULT {
        true => Ok(true),
        false => Ok(false),
    }
}
