use breez_sdk_core::EnvironmentType;
use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct SdkCli {
    /// Optional data dir, default to current directory
    #[clap(name = "data_dir", short = 'd', long = "data_dir")]
    pub(crate) data_dir: Option<String>,
}

#[derive(Parser, Debug, Clone, PartialEq)]
#[clap(rename_all = "snake")]
pub(crate) enum Commands {
    /// Set the API key
    SetAPIKey {
        /// The API key to use        
        key: String,
    },
    /// Set the Environment type
    SetEnv {
        /// The environment to use (staging|production)        
        env: EnvironmentType,
    },
    /// Register a new greenlight node
    RegisterNode {},

    /// Recover a node using the seed only
    RecoverNode {},

    /// Initialize the sdk for existing node
    Init {},

    /// Sync local data with remote node
    Sync {},

    /// Generate a bolt11 invoice
    ReceivePayment { amount: u64, description: String },

    /// Pay using lnurl pay
    LnurlPay { lnurl: String },

    /// Withdraw using lnurl withdraw
    LnurlWithdraw { lnurl: String },

    /// Authenticate using lnurl auth
    LnurlAuth { lnurl: String },

    /// Send on-chain using a reverse swap
    SendOnchain {
        amount_sat: u64,
        onchain_recipient_address: String,
    },

    /// Get the current fees for a potential new reverse swap
    SendOnchainFees {},

    /// Get the current in-progress reverse swap, if one exists
    InProgressReverseSwap {},

    /// Send a lightning payment
    SendPayment {
        bolt11: String,

        #[clap(name = "amount", short = 'a', long = "amt")]
        amount: Option<u64>,
    },

    /// Send a spontaneus (keysend) payment
    SendSpontaneousPayment { node_id: String, amount: u64 },

    /// List all payments
    ListPayments {},

    /// Send on-chain funds to an external address
    Sweep {
        /// The sweep destination address
        to_address: String,

        /// The fee rate for the sweep transaction
        sat_per_byte: u64,
    },

    /// List available LSPs
    ListLsps {},

    /// Connect to an LSP
    ConnectLSP {
        /// The lsp id the sdk should connect to
        lsp_id: String,
    },

    /// The up to date node information
    NodeInfo {},

    /// List fiat currencies
    ListFiat {},

    /// Fetch available fiat rates
    FetchFiatRates {},

    /// Close all LSP channels
    CloseLSPChannels {},

    /// Stop the node
    StopNode {},

    /// List recommended fees baed on the mempool
    RecommendedFees {},

    /// Genreate address to receive onchain
    ReceiveOnchain {},

    /// Get the current in-progress swap if exists
    InProgressSwap {},

    /// List refundable swap addresses
    ListRefundables {},

    /// Broadcase a refund transaction for incomplete swap
    Refund {
        swap_address: String,
        to_address: String,
        sat_per_vbyte: u32,
    },

    /// Execute a low level node command (used for debugging)
    ExecuteDevCommand { command: String },
}
