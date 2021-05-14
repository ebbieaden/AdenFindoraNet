use ruc::*;
use std::env::args;

fn main() {
    let n = pnk!(
        args()
            .nth(1)
            .unwrap_or_else(|| "1".to_owned())
            .parse::<u64>()
    );
    (0..n).for_each(|_| {
        let mnemonic = pnk!(wallet::generate_mnemonic_custom(24, "en"));
        let pubkey =
            pnk!(wallet::restore_keypair_from_mnemonic_default(&mnemonic)).get_pk();
        let pubkey = wallet::public_key_to_base64(&pubkey);
        println!(
            "\x1b[31;01mMnemonic:\x1b[00m {}\n\x1b[31;01mPubKey:\x1b[00m {}\n",
            mnemonic, pubkey
        );
    });
}
