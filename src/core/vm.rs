use anyhow::{anyhow, Result};
use log::debug;

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Push = 0x0a, // 10
    Add = 0x0b,  // 11
}

impl TryFrom<u8> for Instruction {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self> {
        use Instruction::*;
        match value {
            0x0a => Ok(Push),
            0x0b => Ok(Add),
            _ => Err(anyhow!("not a valid instruction")),
        }
    }
}

pub struct VM {
    data: Vec<u8>,
    ip: usize, // instruction pointer
    stack: [u8; 1024],
    sp: i32, // stack pointer
}

impl VM {
    pub fn new(data: Vec<u8>) -> VM {
        Self {
            data,
            ip: 0,
            stack: [0; 1024],
            sp: -1,
        }
    }

    pub fn sp_stack_val(&self) -> Option<u8> {
        if self.sp >= 0 {
            Some(self.stack[self.sp as usize])
        } else {
            None
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            if let Ok(instr) = Instruction::try_from(self.data[self.ip]) {
                self.exec(&instr)?;
            }

            self.ip += 1;

            if self.ip > self.data.len() - 1 {
                break;
            }
        }

        Ok(())
    }

    pub fn exec(&mut self, instr: &Instruction) -> Result<()> {
        match instr {
            Instruction::Push => self.push_stack(self.data[self.ip - 1]),
            Instruction::Add => {
                let a = self.stack[0];
                let b = self.stack[1];
                let c = a + b;
                self.push_stack(c)
            }
        }
        Ok(())
    }

    pub fn push_stack(&mut self, b: u8) {
        self.sp += 1;
        self.stack[self.sp as usize] = b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm() -> Result<()> {
        let mut vm = VM::new(vec![0x01, 0x0a, 0x02, 0x0a, 0x0b]);
        vm.run()?;

        assert_eq!(Some(3), vm.sp_stack_val());

        Ok(())
    }
}
