use super::solidity::*;
use ethereum::TransactionAction;
use ethereum_types::H160;
use fp_utils::ethereum::UnsignedTransaction;
use primitive_types::U256;
use std::path::{Path, PathBuf};
use std::sync::Once;

pub struct ERC20Constructor(pub ContractConstructor);

impl From<ERC20Constructor> for ContractConstructor {
    fn from(c: ERC20Constructor) -> Self {
        c.0
    }
}

pub struct ERC20(pub DeployedContract);

static DOWNLOAD_ONCE: Once = Once::new();

impl ERC20Constructor {
    pub fn load() -> Self {
        Self(ContractConstructor::compile_from_source(
            Self::download_solidity_sources(),
            Self::solidity_artifacts_path(),
            "token/ERC20/presets/ERC20PresetMinterPauser.sol",
            "ERC20PresetMinterPauser",
        ))
    }

    pub fn deploy(&self, name: &str, symbol: &str, nonce: U256) -> UnsignedTransaction {
        let input = self
            .0
            .abi
            .constructor()
            .unwrap()
            .encode_input(
                self.0.code.clone(),
                &[
                    ethabi::Token::String(name.to_string()),
                    ethabi::Token::String(symbol.to_string()),
                ],
            )
            .unwrap();
        UnsignedTransaction {
            nonce,
            gas_price: Default::default(),
            gas_limit: u32::MAX.into(),
            action: TransactionAction::Create,
            value: Default::default(),
            input,
        }
    }

    fn download_solidity_sources() -> PathBuf {
        let sources_dir = Path::new("target").join("openzeppelin-contracts");
        let contracts_dir = sources_dir.join("contracts");
        if contracts_dir.exists() {
            contracts_dir
        } else {
            // Contracts not already present, so download them (but only once, even
            // if multiple tests running in parallel saw `contracts_dir` does not exist).
            DOWNLOAD_ONCE.call_once(|| {
                let url = "https://github.com/OpenZeppelin/openzeppelin-contracts";
                git2::Repository::clone(url, sources_dir).unwrap();
            });
            contracts_dir
        }
    }

    fn solidity_artifacts_path() -> PathBuf {
        Path::new("target").join("solidity_build")
    }
}

impl ERC20 {
    pub fn mint(
        &self,
        recipient: H160,
        amount: U256,
        nonce: U256,
    ) -> UnsignedTransaction {
        let input = self
            .0
            .abi
            .function("mint")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Address(recipient),
                ethabi::Token::Uint(amount),
            ])
            .unwrap();
        UnsignedTransaction {
            nonce,
            gas_price: Default::default(),
            gas_limit: u32::MAX.into(),
            action: TransactionAction::Call(self.0.address),
            value: Default::default(),
            input,
        }
    }

    pub fn transfer(
        &self,
        recipient: H160,
        amount: U256,
        nonce: U256,
        value: U256,
    ) -> UnsignedTransaction {
        let input = self
            .0
            .abi
            .function("transfer")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Address(recipient),
                ethabi::Token::Uint(amount),
            ])
            .unwrap();
        UnsignedTransaction {
            nonce,
            gas_price: Default::default(),
            gas_limit: u32::MAX.into(),
            action: TransactionAction::Call(self.0.address),
            value,
            input,
        }
    }

    pub fn balance_of(&self, address: H160, nonce: U256) -> UnsignedTransaction {
        let input = self
            .0
            .abi
            .function("balanceOf")
            .unwrap()
            .encode_input(&[ethabi::Token::Address(address)])
            .unwrap();
        UnsignedTransaction {
            nonce,
            gas_price: Default::default(),
            gas_limit: u32::MAX.into(),
            action: TransactionAction::Call(self.0.address),
            value: Default::default(),
            input,
        }
    }
}
