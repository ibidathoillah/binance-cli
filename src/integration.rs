//! Stable integration surface for consumers that embed this crate as a
//! submodule or path dependency.

pub use crate::client::BinanceHttpClient;
pub use crate::config::{Config, Credentials, DEFAULT_HOST, ENV_API_KEY, ENV_API_SECRET};
pub use crate::errors::BinanceError;
pub use crate::output::{CommandOutput, OutputFormat};
pub use crate::{
    dispatch, dispatch_non_shell, normalize_pair, normalize_pair_ws, AppContext, Cli, Command,
};

/// Convenience imports for external consumers.
pub mod prelude {
    pub use super::{
        dispatch, dispatch_non_shell, normalize_pair, normalize_pair_ws, AppContext,
        BinanceError, BinanceHttpClient, Cli, Command, CommandOutput, Config, Credentials,
        OutputFormat, DEFAULT_HOST, ENV_API_KEY, ENV_API_SECRET,
    };
}
