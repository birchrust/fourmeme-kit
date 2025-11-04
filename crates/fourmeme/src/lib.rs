pub mod constants;
pub mod parser;

use abi::{
    FourMemeContract::*,
    IERC20::{IERC20Calls, approveCall},
};
use alloy::{
    consensus::{SignableTransaction, TxEnvelope, TypedTransaction},
    eips::Encodable2718,
    hex,
    network::{TransactionBuilder, TxSigner},
    primitives::{Address, U256},
    providers::{DynProvider, Provider},
    rpc::types::{TransactionInput, TransactionReceipt, TransactionRequest},
    signers::local::PrivateKeySigner,
    sol_types::SolInterface,
};
use anyhow::{Error, Result};
use rpc::Rpc;
use std::sync::Arc;

use crate::constants::FOURMEME_CONTRACT;

pub struct FourMeme {
    client: Arc<DynProvider>,
    signer: PrivateKeySigner,
}

impl FourMeme {
    pub async fn init(rpc: Rpc, signer: PrivateKeySigner) -> Result<Self, Error> {
        let client = Arc::new(rpc.client);

        Ok(Self { client, signer })
    }
    /// Handle the buy transaction
    ///
    /// # Arguments
    ///
    /// * `ether_spent` - The amount of ether to spend
    /// * `token` - The address of the token to buy
    /// * `token_bought` - The amount of token to buy
    ///
    #[inline]
    pub async fn buy_token(
        &self,
        ether_spent: U256,
        token: Address,
        gas_price: u128,
    ) -> Result<TransactionReceipt, Error> {
        let buy_tx = TransactionRequest::default()
            .with_to(FOURMEME_CONTRACT)
            .with_value(ether_spent)
            .with_gas_limit(200000_u64)
            .with_gas_price(gas_price)
            .with_input(
                FourMemeContractCalls::buyTokenAMAP {
                    0: buyTokenAMAPCall {
                        tokenAddress: token,
                        funds: ether_spent,
                        minAmount: U256::ZERO,
                    },
                }
                .abi_encode(),
            );

        let pending_tx = self.client.send_transaction(buy_tx).await?;
        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }

    /// Buy the token with a signed transaction
    /// This function is used to bloxroute the submit transaction to the network
    #[inline]
    pub async fn buy_token_signed(
        &self,
        ether_spent: U256,
        token: Address,
        gas_price: u128,
        nonce: u64,
    ) -> Result<String, Error> {
        // Get sender address and chain_id for EIP-155 replay protection
        let sender_address = self.signer.address();
        let chain_id = self.client.get_chain_id().await?;

        let signer = self.signer.clone();

        let buy_tx = TransactionRequest::default()
            .from(sender_address)
            .to(FOURMEME_CONTRACT)
            .value(ether_spent)
            .gas_limit(200000_u64)
            .gas_price(gas_price)
            .nonce(nonce)
            .input(TransactionInput::new(
                FourMemeContractCalls::buyTokenAMAP {
                    0: buyTokenAMAPCall {
                        tokenAddress: token,
                        funds: ether_spent,
                        minAmount: U256::ZERO,
                    },
                }
                .abi_encode()
                .into(),
            ));

        let typed_tx = buy_tx
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

    /// Approve unlimited allowance for the fourmeme contract
    ///
    /// # Arguments
    ///
    /// * `token` - The address of the token to approve
    ///
    #[inline]
    pub async fn approve_token(
        &self,
        token: Address,
        gas_price: u128,
    ) -> Result<TransactionReceipt, Error> {
        let approve_tx = TransactionRequest::default()
            .with_to(token)
            .with_input(
                IERC20Calls::approve {
                    0: approveCall {
                        spender: FOURMEME_CONTRACT,
                        allowance: U256::MAX,
                    },
                }
                .abi_encode(),
            )
            .with_gas_price(gas_price);

        let pending_tx = self.client.send_transaction(approve_tx).await?;
        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }

    /// Sell the token for ether
    ///
    /// # Arguments
    ///
    /// * `token` - The address of the token to sell
    /// * `amount` - The amount of token to sell
    ///
    #[inline]
    pub async fn sell_token(
        &self,
        token: Address,
        amount: U256,
        gas_price: u128,
    ) -> Result<TransactionReceipt, Error> {
        let sell_tx = TransactionRequest::default()
            .with_to(FOURMEME_CONTRACT)
            .with_value(U256::ZERO)
            .with_gas_limit(200000_u64)
            .with_gas_price(gas_price)
            .with_input(
                FourMemeContractCalls::sellToken {
                    0: sellTokenCall {
                        userAddress: token,
                        tokenQty: amount,
                    },
                }
                .abi_encode(),
            );

        let pending_tx = self.client.send_transaction(sell_tx).await?;
        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
    }
}
