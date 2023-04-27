#[derive(Clone)]
pub struct MoonPayConfig {
    pub base_url: String,
    pub api_key: String,
    pub currency_code: String,
    pub color_code: String,
    pub redirect_url: String,
    pub enabled_payment_methods: String,
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::moonpay::moonpay_config::MoonPayConfig;

    pub fn stub_moon_pay_config() -> MoonPayConfig {
        MoonPayConfig {
            base_url: String::from("https://base.url"),
            api_key: String::from("an api key"),
            currency_code: String::from("a currency code"),
            color_code: String::from("a color code"),
            redirect_url: String::from("a redirect url"),
            enabled_payment_methods: String::from("a series of enabled payment methods"),
        }
    }
}
