use anyhow::Result;

use crate::breez_services::BreezServer;
use crate::moonpay::moonpay_url_signer::MoonPayUrlSigner;
use crate::SwapInfo;

#[tonic::async_trait]
pub trait MoonPayApi: Send + Sync {
    async fn sign_moon_pay_url(&self, url_data: &dyn MoonPayUrlData) -> Result<String>;
}

#[tonic::async_trait]
impl MoonPayApi for BreezServer {
    async fn sign_moon_pay_url(&self, url_data: &dyn MoonPayUrlData) -> Result<String> {
        let moonpay_api_key = self.moonpay_api_key.clone().unwrap();
        let signer: &dyn MoonPayUrlSigner = &self.get_signer_client().await?;
        signer
            .sign_moon_pay_url(
                moonpay_api_key.as_str(),
                url_data.bitcoin_address().as_str(),
                url_data.max_allowed_deposit().as_str(),
            )
            .await
    }
}

pub trait MoonPayUrlData: Send + Sync {
    fn bitcoin_address(&self) -> String;
    fn max_allowed_deposit(&self) -> String;
}

impl MoonPayUrlData for SwapInfo {
    fn bitcoin_address(&self) -> String {
        self.bitcoin_address.clone()
    }

    fn max_allowed_deposit(&self) -> String {
        format!("{:.8}", self.max_allowed_deposit as f64 / 100000000.0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::moonpay::moonpay_api::MoonPayUrlData;
    use crate::{SwapInfo, SwapStatus};

    #[test]
    fn test_bitcoin_address_for_swap_info() {
        let swap_info: &dyn MoonPayUrlData = &stub_swap_info();
        assert_eq!(swap_info.bitcoin_address(), "bitcoin_address");
    }

    #[test]
    fn test_max_allowed_deposit_for_swap_info() {
        let swap_info: &dyn MoonPayUrlData = &stub_swap_info();
        assert_eq!(swap_info.max_allowed_deposit(), "9.87654321");
    }

    fn stub_swap_info() -> SwapInfo {
        SwapInfo {
            bitcoin_address: String::from("bitcoin_address"),
            max_allowed_deposit: 987654321,
            // Not used
            created_at: 0,
            lock_height: 0,
            payment_hash: vec![],
            preimage: vec![],
            private_key: vec![],
            public_key: vec![],
            swapper_public_key: vec![],
            script: vec![],
            bolt11: None,
            paid_sats: 0,
            confirmed_sats: 0,
            unconfirmed_sats: 0,
            status: SwapStatus::Initial,
            refund_tx_ids: vec![],
            unconfirmed_tx_ids: vec![],
            confirmed_tx_ids: vec![],
            min_allowed_deposit: 0,
            last_redeem_error: None,
        }
    }
}
