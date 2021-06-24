use crate::crypto::Verify;
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;

/// A type that can be used in structures.
pub trait Member:
    Send + Sync + Sized + Debug + Eq + PartialEq + Clone + 'static
{
}

impl<T: Send + Sync + Sized + Debug + Eq + PartialEq + Clone + 'static> Member for T {}

pub trait TxMsg {
    fn route_path(&self) -> String;

    fn execute(&self) -> Result<()>;

    fn validate(&self) -> Result<()>;

    fn as_any(&self) -> &dyn Any;
}

/// This is unchecked and so can contain a signature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UncheckedTransaction<PublicKey, Call, Signature> {
    /// The signature is to use the PublicKey sign the function if this is a signed transaction.
    pub signature: Option<(PublicKey, Signature)>,
    /// The function that should be called.
    pub function: Call,
}

impl<PublicKey, Call, Signature> UncheckedTransaction<PublicKey, Call, Signature>
where
    PublicKey: Member,
    Call: Member + Serialize,
    Signature: Member + Verify<Signer = PublicKey>,
{
    pub fn new_signed(function: Call, signed: PublicKey, signature: Signature) -> Self {
        Self {
            signature: Some((signed, signature)),
            function,
        }
    }

    pub fn new_unsigned(function: Call) -> Self {
        Self {
            signature: None,
            function,
        }
    }

    pub fn check(self) -> Result<CheckedTransaction<PublicKey, Call>> {
        Ok(match self.signature {
            Some((signed, signature)) => {
                let msg = serde_json::to_vec(&self.function).unwrap();

                if !signature.verify(msg.as_slice(), &signed) {
                    return Err(eg!("bad transaction signature"));
                }

                CheckedTransaction {
                    signed: Some(signed),
                    function: self.function,
                }
            }
            None => CheckedTransaction {
                signed: None,
                function: self.function,
            },
        })
    }
}

/// It has been checked and is good, particularly with regards to the signature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckedTransaction<PublicKey, Call> {
    /// The function signer, if anyone
    pub signed: Option<PublicKey>,

    /// The function that should be called.
    pub function: Call,
}

/// An "executable" piece of information used by the transaction.
pub trait Applyable: Sized + Send + Sync {
    /// Type by which we can execute. Restricts the `UnsignedValidator` type.
    type Call: Member + TxMsg;

    /// Checks to see if this is a valid *transaction*.
    fn validate<V: ValidateUnsigned<Call = Self::Call>>(&self) -> Result<()>;

    /// Executes all necessary logic needed prior to execute and deconstructs into function call,
    /// index and sender.
    fn apply<V: ValidateUnsigned<Call = Self::Call>>(self) -> Result<()>;
}

/// Something that can validate unsigned transactions for the transaction pool.
///
/// Note that any checks done here are only used for determining the validity of
/// the transaction for the transaction pool.
/// During block execution phase one need to perform the same checks anyway,
/// since this function is not being called.
pub trait ValidateUnsigned {
    /// The call to validate
    type Call;

    /// Validate the call right before execute.
    ///
    /// Changes made to storage WILL be persisted if the call returns `Ok`.
    fn pre_execute(call: &Self::Call) -> Result<()> {
        Self::validate_unsigned(call)
    }

    /// Return the validity of the call
    ///
    /// Changes made to storage should be discarded by caller.
    fn validate_unsigned(call: &Self::Call) -> Result<()>;
}

impl<PublicKey, Call> Applyable for CheckedTransaction<PublicKey, Call>
where
    PublicKey: Member,
    Call: Member + TxMsg,
{
    type Call = Call;

    fn validate<U: ValidateUnsigned<Call = Self::Call>>(&self) -> Result<()> {
        if self.signed.is_some() {
            Ok(())
        } else {
            U::validate_unsigned(&self.function)
        }
    }

    fn apply<U: ValidateUnsigned<Call = Self::Call>>(self) -> Result<()> {
        let _maybe_who = if let Some(id) = self.signed {
            Some(id)
        } else {
            U::pre_execute(&self.function)?;
            None
        };
        // TODO
        self.function.execute()
    }
}

pub trait ConvertTransaction<T> {
    fn convert_transaction(&self, _transaction: &[u8]) -> Result<T>;
}
