#[derive(Clone)]
pub(crate) struct MoonPayConfig {
    pub base_url: String,
    pub api_key: String,
    pub currency_code: String,
    pub color_code: String,
    pub redirect_url: String,
    pub enabled_payment_methods: String,
}

pub(crate) fn moonpay_config(moonpay_api_key: &str) -> MoonPayConfig {
    MoonPayConfig {
        base_url: String::from("https://buy.moonpay.io"),
        api_key: String::from(moonpay_api_key),
        currency_code: String::from("btc"),
        color_code: String::from("#055DEB"),
        redirect_url: String::from("https://buy.moonpay.io/transaction_receipt?addFunds=true"),
        enabled_payment_methods: String::from(
            "credit_debit_card,sepa_bank_transfer,gbp_bank_transfer",
        ),
    }
}
