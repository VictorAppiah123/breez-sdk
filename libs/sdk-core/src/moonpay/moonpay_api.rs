use anyhow::Result;

use crate::moonpay::moonpay_config::MoonPayConfig;
use crate::moonpay::moonpay_url_signer::MoonPayUrlSigner;
use crate::SwapInfo;

pub struct MoonPayApi {
    config: MoonPayConfig,
    signer: Box<dyn MoonPayUrlSigner>,
}

impl MoonPayApi {
    pub fn new(config: MoonPayConfig, signer: Box<dyn MoonPayUrlSigner>) -> Self {
        Self { config, signer }
    }

    pub async fn sign_moon_pay_url(&mut self, url_data: &dyn MoonPayUrlData) -> Result<String> {
        self.signer
            .sign_moon_pay_url(
                &self.config,
                url_data.bitcoin_address().as_str(),
                url_data.max_allowed_deposit().as_str(),
            )
            .await
    }
}

pub trait MoonPayUrlData {
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
    use crate::moonpay::moonpay_api::{MoonPayApi, MoonPayUrlData};
    use crate::moonpay::moonpay_config::tests::stub_moon_pay_config;
    use crate::{SwapInfo, SwapStatus};

    #[tokio::test]
    async fn test_sign_moon_pay_url() -> Result<(), Box<dyn std::error::Error>> {
        let mut api = super::MoonPayApi::new(
            stub_moon_pay_config(),
            Box::new(MockMoonPayUrlSigner::default()),
        );
        let url = api
            .sign_moon_pay_url(&MockMoonPayUrlData {
                bitcoin_address: String::from("bitcoin_address"),
                max_allowed_deposit: String::from("max_allowed_deposit"),
            })
            .await?;
        assert_eq!(
            url,
            "https://mock.moonpay?wa=bitcoin_address&ma=max_allowed_deposit"
        );
        Ok(())
    }

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

    #[derive(Default)]
    pub struct MockMoonPayUrlSigner {}

    #[tonic::async_trait]
    impl super::MoonPayUrlSigner for MockMoonPayUrlSigner {
        async fn sign_moon_pay_url(
            &mut self,
            _config: &super::MoonPayConfig,
            _wallet_address: &str,
            _max_quote_currency_amount: &str,
        ) -> super::Result<String> {
            Ok(format!(
                "https://mock.moonpay?wa={}&ma={}",
                _wallet_address, _max_quote_currency_amount
            ))
        }
    }

    #[derive(Default)]
    pub struct MockMoonPayUrlData {
        pub bitcoin_address: String,
        pub max_allowed_deposit: String,
    }

    impl MoonPayUrlData for MockMoonPayUrlData {
        fn bitcoin_address(&self) -> String {
            self.bitcoin_address.clone()
        }

        fn max_allowed_deposit(&self) -> String {
            self.max_allowed_deposit.clone()
        }
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

    pub fn stub_moon_pay_api() -> MoonPayApi {
        MoonPayApi::new(
            stub_moon_pay_config(),
            Box::new(MockMoonPayUrlSigner::default()),
        )
    }
}
