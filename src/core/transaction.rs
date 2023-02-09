use anyhow::Result;

pub struct Transaction {
    pub data: Vec<u8>,
}

impl Transaction {
    pub fn encode_binary(&self, w: &mut dyn std::io::Write) -> Result<()> {
        Ok(())
    }

    pub fn decode_binary(&mut self, r: &mut dyn std::io::Read) -> Result<()> {
        Ok(())
    }
}
