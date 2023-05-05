use anyhow::Result;
use reqwest::Url;

use crate::grpc::signer_client::SignerClient;
use crate::grpc::SignUrlRequest;
use crate::moonpay::moonpay_config::MoonPayConfig;

#[tonic::async_trait]
pub trait MoonPayUrlSigner: Send + Sync {
    async fn sign_moon_pay_url(
        &mut self,
        moon_pay_config: &MoonPayConfig,
        wallet_address: &str,
        max_quote_currency_amount: &str,
    ) -> Result<String>;
}

#[tonic::async_trait]
impl MoonPayUrlSigner for SignerClient<tonic::transport::Channel> {
    async fn sign_moon_pay_url(
        &mut self,
        config: &MoonPayConfig,
        wallet_address: &str,
        max_quote_currency_amount: &str,
    ) -> Result<String> {
        let url = Url::parse_with_params(
            &config.base_url,
            &[
                ("apiKey", &config.api_key),
                ("currencyCode", &config.currency_code),
                ("colorCode", &config.color_code),
                ("redirectURL", &config.redirect_url),
                ("enabledPaymentMethods", &config.enabled_payment_methods),
                ("walletAddress", &wallet_address.to_string()),
                (
                    "maxQuoteCurrencyAmount",
                    &max_quote_currency_amount.to_string(),
                ),
            ],
        )?;
        let signed_url = self
            .sign_url(SignUrlRequest {
                base_url: config.base_url.clone(),
                query_string: format!("?{}", url.query().unwrap()),
            })
            .await?
            .into_inner()
            .full_url;
        Ok(signed_url)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::io::{Error, ErrorKind};

    use reqwest::Url;
    use tonic::transport::{Endpoint, Server, Uri};

    use futures::stream::iter;
    use tower::service_fn;

    use crate::grpc::signer_client::SignerClient;
    use crate::grpc::signer_server::{Signer, SignerServer};
    use crate::grpc::{SignUrlRequest, SignUrlResponse};
    use crate::moonpay::moonpay_config::tests::stub_moon_pay_config;
    use crate::moonpay::moonpay_url_signer::MoonPayUrlSigner;

    #[tokio::test]
    async fn test_sign_moon_pay_url() -> Result<(), Box<dyn std::error::Error>> {
        let (client, server) = tokio::io::duplex(1024);
        let signer = MockSigner::default();
        tokio::spawn(async move {
            Server::builder()
                .add_service(SignerServer::new(signer))
                .serve_with_incoming(iter(vec![Ok::<_, std::io::Error>(server)]))
                .await
        });
        let mut client = Some(client);
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(move |_: Uri| {
                let client = client.take();
                async move {
                    if let Some(client) = client {
                        Ok(client)
                    } else {
                        Err(Error::new(ErrorKind::Other, "Client already taken"))
                    }
                }
            }))
            .await?;

        let config = stub_moon_pay_config();
        let wallet_address = "a wallet address";
        let max_quote_currency_amount = "a max quote currency amount";

        let mut signer: Box<dyn MoonPayUrlSigner> = Box::new(SignerClient::new(channel));
        let signed_url = signer
            .sign_moon_pay_url(&config, wallet_address, max_quote_currency_amount)
            .await?;
        let parsed = Url::parse(&signed_url)?;

        let query_pairs = parsed.query_pairs().into_owned().collect::<HashMap<_, _>>();
        assert_eq!(parsed.host_str(), Some("base.url"));
        assert_eq!(parsed.path(), "/");
        assert_eq!(query_pairs.get("apiKey"), Some(&config.api_key));
        assert_eq!(query_pairs.get("currencyCode"), Some(&config.currency_code));
        assert_eq!(query_pairs.get("colorCode"), Some(&config.color_code));
        assert_eq!(query_pairs.get("redirectURL"), Some(&config.redirect_url));
        assert_eq!(
            query_pairs.get("enabledPaymentMethods"),
            Some(&config.enabled_payment_methods),
        );
        assert_eq!(
            query_pairs.get("walletAddress"),
            Some(&String::from(wallet_address))
        );
        assert_eq!(
            query_pairs.get("maxQuoteCurrencyAmount"),
            Some(&String::from(max_quote_currency_amount)),
        );
        assert_eq!(
            query_pairs.get("signature"),
            Some(&String::from("a_server_signature")),
        );
        Ok(())
    }

    #[derive(Default)]
    pub struct MockSigner {}

    #[tonic::async_trait]
    impl Signer for MockSigner {
        async fn sign_url(
            &self,
            request: tonic::Request<SignUrlRequest>,
        ) -> Result<tonic::Response<SignUrlResponse>, tonic::Status> {
            let message = request.into_inner();
            let reply = SignUrlResponse {
                full_url: format!(
                    "{}{}&signature=a_server_signature",
                    message.base_url, message.query_string,
                ),
            };
            Ok(tonic::Response::new(reply))
        }
    }
}
