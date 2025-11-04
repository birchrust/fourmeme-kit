pub mod constants;
pub mod parser;

use abi::IERC20::{IERC20Calls, approveCall};
use alloy::{
    consensus::{SignableTransaction, TxEnvelope, TypedTransaction},
    eips::Encodable2718,
    hex,
    network::{TransactionBuilder, TxSigner},
    primitives::{Address, U256, address},
    providers::{DynProvider, Provider},
    rpc::types::{TransactionInput, TransactionReceipt, TransactionRequest},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::{SolCall, SolInterface},
};
use anyhow::Error;
use rpc::Rpc;
use std::sync::Arc;

/// PancakeSwap Router
pub const PANCAKESWAP_ROUTER: Address = address!("0x10ED43C718714eb63d5aA57B78B54704E256024E");

/// WBNB
pub const WBNB: Address = address!("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c");

sol! {
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    contract PancakeSwapRouter {
        function swapExactETHForTokens(
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external payable;

        function swapExactTokensForETH(
            uint256 amountIn,
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external;
    }
}

#[derive(Clone)]
pub struct Pancake {
    client: Arc<DynProvider>,
    receiver: Address,
    signer: PrivateKeySigner,
}

impl Pancake {
    pub async fn init(rpc: Rpc, signer: PrivateKeySigner) -> Result<Self, Error> {
        let client = Arc::new(rpc.client);
        let receiver = rpc.sender_address;
        Ok(Self {
            client,
            receiver,
            signer,
        })
    }

    /// Use ether to buy tokens
    ///
    /// # Arguments
    ///
    /// * `token` - The address of the token to buy
    /// * `ether_spent` - The amount of ether to spend
    ///
    /// # Returns
    ///
    /// * `TransactionReceipt` - The receipt of the transaction
    #[inline]
    pub async fn swap_exact_ethfor_tokens(
        &self,
        token: Address,
        ether_spent: U256,
    ) -> Result<TransactionReceipt, Error> {
        let path = vec![WBNB, token];

        let deadline = U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                + 300,
        );
        let pending_tx = PancakeSwapRouter::new(PANCAKESWAP_ROUTER, &self.client)
            .swapExactETHForTokens(U256::ZERO, path, self.receiver, deadline)
            .value(ether_spent)
            .send()
            .await?;

        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }

    /// Use ether to buy tokens with a signed transaction
    /// This function returns a signed transaction hex string without submitting it
    ///
    /// # Arguments
    ///
    /// * `token` - The address of the token to buy
    /// * `ether_spent` - The amount of ether to spend
    /// * `gas_price` - Gas price in wei
    /// * `nonce` - Transaction nonce
    ///
    /// # Returns
    ///
    /// * `String` - The signed transaction hex string
    #[inline]
    pub async fn swap_exact_ethfor_tokens_signed(
        &self,
        token: Address,
        ether_spent: U256,
        gas_price: u128,
        nonce: u64,
    ) -> Result<String, Error> {
        // Get sender address and chain_id for EIP-155 replay protection
        let sender_address = self.signer.address();
        let chain_id = self.client.get_chain_id().await?;

        let signer = self.signer.clone();

        let path = vec![WBNB, token];
        let deadline = U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                + 300,
        );

        // Build the transaction using ABI encoding
        let swap_tx = TransactionRequest::default()
            .from(sender_address)
            .to(PANCAKESWAP_ROUTER)
            .value(ether_spent)
            .gas_limit(300000_u64)
            .gas_price(gas_price)
            .nonce(nonce)
            .input(TransactionInput::new(
                PancakeSwapRouter::swapExactETHForTokensCall {
                    amountOutMin: U256::ZERO,
                    path: path,
                    to: self.receiver,
                    deadline: deadline,
                }
                .abi_encode()
                .into(),
            ));

        let typed_tx = swap_tx
            .build_typed_tx()
            .map_err(|e| Error::msg(format!("Failed to build typed transaction: {:?}", e)))?;

        let signed_envelope: TxEnvelope = match typed_tx {
            TypedTransaction::Legacy(mut tx) => {
                tx.chain_id = Some(chain_id);
                let sig = signer
                    .sign_transaction(&mut tx)
                    .await
                    .map_err(|e| Error::msg(format!("Failed to sign transaction: {:?}", e)))?;
                tx.into_signed(sig).into()
            }
            TypedTransaction::Eip1559(mut tx) => {
                tx.chain_id = chain_id;
                let sig = signer
                    .sign_transaction(&mut tx)
                    .await
                    .map_err(|e| Error::msg(format!("Failed to sign transaction: {:?}", e)))?;
                tx.into_signed(sig).into()
            }
            _ => {
                return Err(Error::msg("Unsupported transaction type"));
            }
        };

        let raw_tx_bytes: Vec<u8> = signed_envelope.encoded_2718();
        let raw_tx_hex = hex::encode(&raw_tx_bytes);

        Ok(raw_tx_hex)
    }

    /// Sell tokens for ether
    ///
    /// # Arguments
    ///
    /// * `token` - The address of the token to sell
    /// * `tokens_spent` - The amount of tokens to sell
    ///
    /// # Returns
    ///
    /// * `TransactionReceipt` - The receipt of the transaction
    #[inline]
    pub async fn swap_exact_tokensfor_eth(
        &self,
        token: Address,
        tokens_spent: U256,
    ) -> Result<TransactionReceipt, Error> {
        let path = vec![token, WBNB];
        let deadline = U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                + 300,
        );
        let pending_tx = PancakeSwapRouter::new(PANCAKESWAP_ROUTER, &self.client)
            .swapExactTokensForETH(tokens_spent, U256::ZERO, path, self.receiver, deadline)
            .send()
            .await?;

        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }

    /// Approve the pancake swap router to spend the token
    #[inline]
    pub async fn approve_token(&self, token: Address) -> Result<TransactionReceipt, Error> {
        let approve_tx = TransactionRequest::default()
            .with_to(token)
            .with_input(
                IERC20Calls::approve {
                    0: approveCall {
                        spender: PANCAKESWAP_ROUTER,
                        allowance: U256::MAX,
                    },
                }
                .abi_encode(),
            )
            .with_gas_limit(10_000_000_000_u64);

        let pending_tx = self.client.send_transaction(approve_tx).await?;
        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }
}
