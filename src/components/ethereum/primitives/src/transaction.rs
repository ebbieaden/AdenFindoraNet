use crate::{
    context::Context,
    crypto::{IdentifyAccount, Verify},
};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// A type that can be used in structures.
pub trait Member:
    Send + Sync + Sized + Debug + Eq + PartialEq + Clone + 'static
{
}

impl<T: Send + Sync + Sized + Debug + Eq + PartialEq + Clone + 'static> Member for T {}

/// A action (module function and argument values) that can be executed.
pub trait Executable {
    type Origin;

    /// Actually execute this action and return the result of it.
    fn execute(self, origin: Option<Self::Origin>, ctx: Context) -> Result<()>;
}

/// This is unchecked and so can contain a signature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UncheckedTransaction<Address, Call, Signature> {
    /// The signature is to use the Address sign the function if this is a signed transaction.
    pub signature: Option<(Address, Signature)>,
    /// The function that should be called.
    pub function: Call,
}

impl<Address, Call, Signature> UncheckedTransaction<Address, Call, Signature> {
    pub fn new_signed(function: Call, signed: Address, signature: Signature) -> Self {
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
}

impl<Address, Call, Signature> UncheckedTransaction<Address, Call, Signature>
where
    Call: Serialize,
    Signature: Verify,
    <Signature as Verify>::Signer: IdentifyAccount<AccountId = Address>,
{
    pub fn check(self) -> Result<CheckedTransaction<Address, Call>> {
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
pub struct CheckedTransaction<Address, Call> {
    /// The function signer, if anyone
    pub signed: Option<Address>,

    /// The function that should be called.
    pub function: Call,
}

/// An "executable" action used by the transaction.
pub trait Applyable {
    /// Type by which we can execute. Restricts the `UnsignedValidator` type.
    type Call: Executable;

    /// Checks to see if this is a valid *transaction*.
    fn validate<V: ValidateUnsigned<Call = Self::Call>>(
        &self,
        ctx: Context,
    ) -> Result<()>;

    /// Executes all necessary logic needed prior to execute and deconstructs into function call,
    /// index and sender.
    fn apply<V: ValidateUnsigned<Call = Self::Call>>(self, ctx: Context) -> Result<()>;
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
    fn pre_execute(call: &Self::Call, ctx: Context) -> Result<()> {
        Self::validate_unsigned(call, ctx)
    }

    /// Return the validity of the call
    ///
    /// Changes made to storage should be discarded by caller.
    fn validate_unsigned(call: &Self::Call, ctx: Context) -> Result<()>;
}

impl<Address, Call> Applyable for CheckedTransaction<Address, Call>
where
    Call: Executable<Origin = Address>,
{
    type Call = Call;

    fn validate<U: ValidateUnsigned<Call = Self::Call>>(
        &self,
        ctx: Context,
    ) -> Result<()> {
        if self.signed.is_some() {
            Ok(())
        } else {
            U::validate_unsigned(&self.function, ctx)
        }
    }

    fn apply<U: ValidateUnsigned<Call = Self::Call>>(self, ctx: Context) -> Result<()> {
        let maybe_who = if let Some(id) = self.signed {
            Some(id)
        } else {
            U::pre_execute(&self.function, ctx.clone())?;
            None
        };
        // TODO
        self.function.execute(maybe_who, ctx)
    }
}
