mod basic;
mod client;
mod genesis;
mod impls;

use fp_core::{
    context::Context,
    crypto::Address,
    mint_output::MintOutput,
    module::AppModule,
    transaction::{ActionResult, Executable, ValidateUnsigned},
};
use fp_traits::account::AccountAsset;
use ledger::data_model::ASSET_TYPE_FRA;
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, marker::PhantomData};
use zei::xfr::structs::AssetType;

pub trait Config {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transfer((Address, u128)),
    TransferToUTXO(Vec<MintOutput>),
}

mod storage {
    use fp_core::{account::SmartAccount, crypto::Address, mint_output::MintOutput};
    use fp_storage::*;

    // Store account information under all account addresses
    generate_storage!(Account, AccountStore => Map<Address, SmartAccount>);
    // Store MintOutputDefine
    generate_storage!(Account, MintOutputs => Value<Vec<MintOutput>>);
}

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "account".to_string(),
            phantom: Default::default(),
        }
    }

    fn add_mint(ctx: &Context, mut outputs: Vec<MintOutput>) -> Result<()> {
        let ops = if let Some(mut ori_outputs) =
            storage::MintOutputs::get(ctx.store.clone())
        {
            ori_outputs.append(&mut outputs);
            ori_outputs
        } else {
            outputs
        };
        storage::MintOutputs::put(ctx.store.clone(), ops);
        Ok(())
    }

    pub fn consume_mint(ctx: &Context, size: usize) -> Result<Vec<MintOutput>> {
        let res = if let Some(mut outputs) = storage::MintOutputs::get(ctx.store.clone())
        {
            if outputs.len() <= size {
                outputs
            } else {
                let vec2 = outputs.split_off(size - outputs.len());
                storage::MintOutputs::put(ctx.store.clone(), vec2);
                outputs
            }
        } else {
            Vec::new()
        };
        Ok(res)
    }
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModule for App<C> {}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: &Context,
    ) -> Result<ActionResult> {
        match call {
            Action::Transfer((dest, balance)) => {
                if let Some(sender) = origin {
                    Self::transfer(ctx, &sender, &dest, balance)?;
                    Ok(ActionResult::default())
                } else {
                    Err(eg!("invalid transaction origin"))
                }
            }
            Action::TransferToUTXO(outputs) => {
                if let Some(sender) = origin {
                    let mut asset_amount = 0;
                    let mut asset_map = HashMap::new();
                    for output in &outputs {
                        if output.asset == ASSET_TYPE_FRA {
                            asset_amount += output.amount;
                        } else {
                            if let Some(amount) = asset_map.get_mut(&output.asset) {
                                *amount += output.amount;
                            } else {
                                asset_map.insert(output.asset, output.amount);
                            }
                        }
                    }

                    log::info!(target: "account", "this tx's amount is: FRA: {}, OTHER: {:?}", asset_amount, asset_map);

                    let sa = Self::account_of(ctx, &sender).c(d!("no account!"))?;

                    if sa.balance < asset_amount as u128 {
                        return Err(eg!("insufficient balance fra"));
                    }

                    for (k, v) in asset_map.iter() {
                        if let Some(asset_balance) = sa.assets.get(&k) {
                            if asset_balance < &(v.clone() as u128) {
                                return Err(eg!("insufficient balance"));
                            }
                        } else {
                            return Err(eg!("insufficient balance, no asset"));
                        }
                    }

                    for (k, v) in asset_map.into_iter() {
                        Self::burn(ctx, &sender, v as u128, k)?;
                    }
                    Self::add_mint(ctx, outputs)?;
                    Ok(ActionResult::default())
                } else {
                    Err(eg!("invalid transaction origin"))
                }
            }
        }
    }
}

impl<C: Config> ValidateUnsigned for App<C> {
    type Call = Action;

    fn validate_unsigned(call: &Self::Call, _ctx: &Context) -> Result<()> {
        match call {
            Action::TransferToUTXO(_outputs) => Ok(()),
            _ => Err(eg!("invalid unsigned transaction")),
        }
    }
}
