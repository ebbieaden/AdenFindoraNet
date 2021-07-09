use fp_core::context::Context;

pub trait AccountInfo<Address> {
    /// The balance of `who`.
    fn balance(ctx: &Context, who: &Address) -> u128;

    /// The nonce of `who`.
    fn nonce(ctx: &Context, who: &Address) -> u64;
}
