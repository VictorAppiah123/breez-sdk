use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::boltzswap::BoltzApiCreateReverseSwapResponse;
use crate::boltzswap::BoltzApiReverseSwapStatus::SwapCreated;
use crate::chain::{get_utxos, ChainService, MempoolSpace};
use crate::models::ReverseSwapperAPI;
use crate::{
    BreezEvent, ReverseSwapInfo, ReverseSwapInfoCached, ReverseSwapPairInfo, ReverseSwapStatus,
};
use anyhow::{anyhow, Result};
use bitcoin::blockdata::constants::WITNESS_SCALE_FACTOR;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::util::sighash::SighashCache;
use bitcoin::{
    Address, AddressType, EcdsaSighashType, Script, Sequence, Transaction, TxIn, TxOut, Witness,
};
use bitcoin_hashes::hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateReverseSwapResponse {
    id: String,

    /// HODL invoice that has to be paid, for the Boltz service to lock up the funds
    invoice: String,

    /// Redeem script from which the lock address is derived. Can be used to check that the Boltz
    /// service didn't create an address without an HTLC.
    redeem_script: String,

    /// Amount of sats which will be locked
    onchain_amount: u64,

    /// Block height at which the reverse swap will be considered cancelled
    timeout_block_height: u32,

    /// Address to which the funds will be locked
    lockup_address: String,
}

/// This struct is responsible for sending to an onchain address using lightning payments.
/// It uses internally an implementation of [ReverseSwapperAPI] that represents Boltz reverse swapper service.
pub(crate) struct BTCSendSwap {
    _network: bitcoin::Network,
    pub(crate) reverse_swapper_api: Arc<dyn ReverseSwapperAPI>,
    persister: Arc<crate::persist::db::SqliteStorage>,
    chain_service: Arc<dyn ChainService>,
}

impl BTCSendSwap {
    pub(crate) fn new(
        _network: bitcoin::Network,
        reverse_swapper_api: Arc<dyn ReverseSwapperAPI>,
        persister: Arc<crate::persist::db::SqliteStorage>,
        chain_service: Arc<MempoolSpace>,
    ) -> Self {
        Self {
            _network,
            reverse_swapper_api,
            persister,
            chain_service,
            //payment_sender,
        }
    }

    fn validate_create_reverse_swap(onchain_destination_address: &str) -> Result<()> {
        Address::from_str(onchain_destination_address)
            .map(|_| ())
            .map_err(|_e| anyhow!("Invalid destination address"))
    }

    pub(crate) async fn create_reverse_swap(
        &self,
        amount_sat: u64,
        onchain_destination_address: String,
        pair_hash: String,
        routing_node: String,
    ) -> Result<ReverseSwapInfo> {
        Self::validate_create_reverse_swap(&onchain_destination_address)?;

        let reverse_swap_private_data = crate::swap::create_swap_keys()?;
        let boltz_response = self
            .reverse_swapper_api
            .create_reverse_swap(
                amount_sat,
                reverse_swap_private_data.preimage_hash_bytes().to_hex(),
                reverse_swap_private_data.public_key()?.to_hex(),
                pair_hash.clone(),
                routing_node,
            )
            .await?;

        match boltz_response {
            BoltzApiCreateReverseSwapResponse::BoltzApiSuccess(response) => {
                // Successful reverse swap initiated

                let rev_swap_info = ReverseSwapInfo {
                    created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
                    destination_address: onchain_destination_address,
                    hodl_bolt11: response.invoice,
                    local_preimage: reverse_swap_private_data.preimage,
                    local_private_key: reverse_swap_private_data.priv_key,
                    id: response.id,
                    boltz_api_status: SwapCreated,
                    redeem_script: response.redeem_script,
                    cache: ReverseSwapInfoCached {
                        lockup_address: response.lockup_address,
                        onchain_amount_sat: response.onchain_amount,
                    },
                };

                self.persister.insert_reverse_swap(&rev_swap_info)?;
                Ok(rev_swap_info)
            }
            BoltzApiCreateReverseSwapResponse::BoltzApiError { error } => {
                // Error reported by the Boltz API
                Err(anyhow!(error))
            }
        }
    }

    pub(crate) async fn on_event(&self, e: BreezEvent) -> Result<()> {
        match e {
            BreezEvent::NewBlock { block: _ } => self.execute_pending_reverse_swaps().await,
            _ => Ok(()),
        }
    }

    /// Builds and signs claim tx
    async fn create_claim_tx(&self, rs: &ReverseSwapInfo) -> Result<Transaction> {
        let lockup_addr_str = Address::from_str(&rs.cache.lockup_address)?;
        let destination_addr = Address::from_str(&rs.destination_address)?;
        let redeem_script = Script::from_hex(&rs.redeem_script)?;

        match lockup_addr_str.address_type() {
            Some(AddressType::P2wsh) => {
                let txs = self
                    .chain_service
                    .address_transactions(rs.cache.lockup_address.clone())
                    .await?;
                let utxos = get_utxos(rs.cache.lockup_address.clone(), txs)?;

                let confirmed_amount: u64 = utxos
                    .confirmed
                    .iter()
                    .fold(0, |accum, item| accum + item.value as u64);

                let txins: Vec<TxIn> = utxos
                    .confirmed
                    .iter()
                    .map(|utxo| TxIn {
                        previous_output: utxo.out,
                        script_sig: Script::new(),
                        sequence: Sequence(0),
                        witness: Witness::default(),
                    })
                    .collect();

                let tx_out: Vec<TxOut> = vec![TxOut {
                    value: confirmed_amount,
                    script_pubkey: destination_addr.script_pubkey(),
                }];

                // construct the transaction
                let mut tx = Transaction {
                    version: 2,
                    lock_time: bitcoin::PackedLockTime(0),
                    input: txins.clone(),
                    output: tx_out,
                };

                let recommended_fees = self.chain_service.recommended_fees().await?;
                let sat_per_vbyte = recommended_fees.half_hour_fee; // TODO Configurable

                let redeem_script_bytes =
                    bitcoin::psbt::serialize::Serialize::serialize(&redeem_script);

                // Based on https://github.com/breez/boltz/blob/master/boltz.go#L31
                let refund_witness_input_size: u32 = 1 + 1 + 8 + 73 + 1 + 32 + 1 + 100;
                let tx_weight = tx.strippedsize() as u32 * WITNESS_SCALE_FACTOR as u32
                    + refund_witness_input_size * txins.len() as u32;
                let fees: u64 = (tx_weight * sat_per_vbyte / WITNESS_SCALE_FACTOR as u32) as u64;
                tx.output[0].value = confirmed_amount - fees;

                let scpt = Secp256k1::signing_only();

                // Sign inputs (iterate, even though we only have one input)
                let mut signed_inputs: Vec<TxIn> = Vec::new();
                for (index, input) in tx.input.iter().enumerate() {
                    let mut signer = SighashCache::new(&tx);
                    let sig = signer.segwit_signature_hash(
                        index,
                        &redeem_script,
                        utxos.confirmed[index].value as u64,
                        EcdsaSighashType::All,
                    )?;
                    let msg = Message::from_slice(&sig[..])?;
                    let secret_key = SecretKey::from_slice(rs.local_private_key.as_slice())?;
                    let sig = scpt.sign_ecdsa(&msg, &secret_key);

                    let mut sigvec = sig.serialize_der().to_vec();
                    sigvec.push(EcdsaSighashType::All as u8);

                    let witness: Vec<Vec<u8>> = vec![
                        sigvec,
                        rs.local_preimage.clone(),
                        redeem_script_bytes.clone(),
                    ];

                    let mut signed_input = input.clone();
                    let w = Witness::from_vec(witness);
                    signed_input.witness = w;
                    signed_inputs.push(signed_input);
                }
                tx.input = signed_inputs;

                Ok(tx)
            }
            Some(addr_type) => Err(anyhow!("Unexpected lock address type: {addr_type:?}")),
            None => Err(anyhow!("Could not determine lock address type")),
        }
    }

    pub(crate) async fn execute_pending_reverse_swaps(&self) -> Result<()> {
        let monitored = self.refresh_monitored_reverse_swaps().await?;
        info!("Found {} monitored reverse swaps", monitored.len());

        // Depending on the new state, decide next steps and transition to the new state
        for rs in monitored {
            info!("Checking monitored {rs:?}");

            if rs.status() == ReverseSwapStatus::LockTxConfirmed {
                info!("Lock tx is confirmed, preparing claim tx");
                let claim_tx = self.create_claim_tx(&rs).await?;
                let claim_tx_broadcast_res = self
                    .chain_service
                    .broadcast_transaction(bitcoin::psbt::serialize::Serialize::serialize(
                        &claim_tx,
                    ))
                    .await;
                info!("Broadcast claim tx result: {claim_tx_broadcast_res:?}");
            }
        }

        Ok(())
    }

    /// Update the state of monitored reverse swaps, and return them with the updated status
    async fn refresh_monitored_reverse_swaps(&self) -> Result<Vec<ReverseSwapInfo>> {
        let to_check = self.list_monitored()?;
        for rs in to_check {
            let id = rs.id.clone();
            let new_boltz_status = self.reverse_swapper_api.get_swap_status(id.clone()).await?;

            match self.persister.update_reverse_swap_boltz_status(&id, &new_boltz_status) {
                Ok(_) => info!("Updated Boltz status for reverse swap ID {id} to {new_boltz_status:?}"),
                Err(e) => error!("Failed to update Boltz status for reverse swap ID {id} to {new_boltz_status:?}: {e}"),
            }
        }
        self.list_monitored()
    }

    pub fn list_monitored(&self) -> Result<Vec<ReverseSwapInfo>> {
        self.persister.get_monitored_reverse_swaps()
    }

    /// See [ReverseSwapperAPI::reverse_swap_pair_info]
    pub(crate) async fn reverse_swap_pair_info(&self) -> Result<ReverseSwapPairInfo> {
        self.reverse_swapper_api.reverse_swap_pair_info().await
    }
}
