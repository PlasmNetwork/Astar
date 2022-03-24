//! Astar XCM CLI handlers.

use crate::cli::*;

use cumulus_primitives_core::ParaId;
use polkadot_primitives::v0::AccountId;
use polkadot_parachain::primitives::{Sibling, AccountIdConversion};
use xcm_builder::SiblingParachainConvertsVia;
use xcm_executor::traits::Convert;
use xcm::latest::prelude::*;
use clap::Parser;

/// CLI error type.
pub type Error = String;

/// Parse command line arguments into service configuration.
pub fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Subcommand::ParachainAccount(cmd)) => {
            let parachain_account = if cmd.sibling {
                let location = MultiLocation { parents: 1, interior: X1(Parachain(cmd.parachain_id)) };
                SiblingParachainConvertsVia::<Sibling, AccountId>::convert_ref(
                    &location
                ).unwrap()
            } else {
                let para_id = ParaId::from(cmd.parachain_id);
                AccountIdConversion::<AccountId>::into_account(&para_id)
            };
            println!("{}", parachain_account);
        }
        Some(Subcommand::AssetId(cmd)) => {
        }
        None => {}
    }
    Ok(())
}
