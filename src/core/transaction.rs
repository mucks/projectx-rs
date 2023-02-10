use serde::{Deserialize, Serialize};

use crate::crypto::{PublicKey, Signature};

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub data: Vec<u8>,

    public_key: PublicKey,
    signature: Signature,
}
