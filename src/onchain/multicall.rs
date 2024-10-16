use std::sync::Arc;

use alloy::{
    dyn_abi::SolType,
    network::Ethereum,
    primitives::{Address, U256},
    providers::Provider,
    sol,
    sol_types::SolCall,
    transports::Transport,
};
use itertools::Itertools;

use super::{
    client::IERC20::balanceOfCall, constants::MULTICALL_CONTRACT_ADDRESS, types::token::Token,
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
}

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
    let out = multicall_instance
        .aggregate3(calls)
        .call()
        .await?
        .returnData
        .iter()
        .map(|balance| {
            <sol! { uint256 }>::abi_decode(&balance.returnData, false).unwrap_or(U256::ZERO)
        })
        .collect::<Vec<_>>();

    Ok(out)
}
