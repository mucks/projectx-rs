//TODO: optimize this vm!

use std::ops::{Add, Sub};

use anyhow::{anyhow, Result};
use log::debug;

use super::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    PushInt = 0x0a,
    Add = 0x0b,
    PushByte = 0x0c,
    Pack = 0x0d,
    Sub = 0x0e,
    Store = 0x0f,
}

impl TryFrom<u8> for Instruction {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<Self> {
        use Instruction::*;
        let v = match value {
            0x0a => PushInt,
            0x0b => Add,
            0x0c => PushByte,
            0x0d => Pack,
            0x0e => Sub,
            0x0f => Store,
            _ => return Err(anyhow!("not a valid instruction")),
        };
        Ok(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackItem {
    Byte(u8),
    Bytes4([u8; 4]),
    Bytes8([u8; 8]),
    Bytes16([u8; 16]),
    Bytes32([u8; 32]),
    Bytes64([u8; 64]),
    //Bytes(Vec<u8>),
    Int(i32),
}

impl StackItem {
    pub fn to_string(self) -> Result<String> {
        let bytes = self.to_bytes();
        let mut new_bytes = vec![];
        for b in bytes {
            if b != 0 {
                new_bytes.push(b);
            }
        }

        let s = String::from_utf8(new_bytes)?;
        Ok(s)
    }

    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            StackItem::Bytes4(b) => b.to_vec(),
            StackItem::Bytes8(b) => b.to_vec(),
            StackItem::Bytes16(b) => b.to_vec(),
            StackItem::Bytes32(b) => b.to_vec(),
            StackItem::Bytes64(b) => b.to_vec(),
            StackItem::Int(b) => vec![b as u8],
            StackItem::Byte(b) => vec![b],
        }
    }

    pub fn add(self, rhs: Self) -> Result<Self> {
        let err: Result<Self> = Err(anyhow!("could not add {:?} + {:?}", self, rhs));
        let s = match self {
            StackItem::Byte(a) => match rhs {
                StackItem::Byte(b) => StackItem::Byte(a + b),
                StackItem::Int(b) => StackItem::Int(a as i32 + b),
                _ => return err,
            },
            StackItem::Int(a) => match rhs {
                StackItem::Byte(b) => StackItem::Int(a + b as i32),
                StackItem::Int(b) => StackItem::Int(a + b),
                _ => return err,
            },
            _ => {
                return err;
            }
        };
        Ok(s)
    }

    pub fn sub(self, rhs: Self) -> Result<Self> {
        let err: Result<Self> = Err(anyhow!("could not sub {:?} - {:?}", self, rhs));
        let s = match self {
            StackItem::Byte(a) => match rhs {
                StackItem::Byte(b) => StackItem::Byte(a - b),
                StackItem::Int(b) => StackItem::Int(a as i32 - b),
                _ => return err,
            },
            StackItem::Int(a) => match rhs {
                StackItem::Byte(b) => StackItem::Int(a - b as i32),
                StackItem::Int(b) => StackItem::Int(a - b),
                _ => return err,
            },
            _ => {
                return err;
            }
        };
        Ok(s)
    }
}

impl TryInto<usize> for StackItem {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<usize, Self::Error> {
        match self {
            Self::Byte(b) => Ok(b as usize),
            Self::Int(b) => Ok(b as usize),
            _ => Err(anyhow!("can't convert {:?} to usize", self)),
        }
    }
}

impl TryInto<u8> for StackItem {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<u8, Self::Error> {
        match self {
            Self::Byte(b) => Ok(b),
            Self::Int(b) => Ok(b as u8),
            _ => Err(anyhow!("can't convert {:?} to u8", self)),
        }
    }
}

impl StackItem {}

impl Default for StackItem {
    fn default() -> Self {
        Self::Byte(0)
    }
}

#[derive(Debug)]
pub struct Stack<const N: usize> {
    data: [StackItem; N],
    sp: usize,
}

impl<const N: usize> Stack<N> {
    pub fn new() -> Self {
        Self {
            data: [StackItem::default(); N],
            sp: 0,
        }
    }

    pub fn pop(&mut self) -> StackItem {
        let val = self.data[0];

        // TODO: optimize this?
        for i in 1..N {
            self.data[i - 1] = self.data[i];
        }

        self.sp = self.sp.saturating_sub(1);

        val
    }

    pub fn push(&mut self, item: StackItem) {
        self.data[self.sp] = item;
        self.sp += 1;
    }
}

pub struct VM<'a> {
    data: Vec<u8>,
    ip: usize, // instruction pointer
    pub stack: Stack<128>,
    contract_state: &'a mut State,
}

impl<'a> VM<'a> {
    pub fn new(data: Vec<u8>, contract_state: &'a mut State) -> VM<'a> {
        Self {
            data,
            ip: 0,
            stack: Stack::new(),
            contract_state,
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

    fn get_bytes<const N: usize>(&mut self, n: usize) -> Result<[u8; N]> {
        let mut b = [0_u8; N];
        for i in 0..n {
            b[i] = self.stack.pop().try_into()?;
        }
        Ok(b)
    }

    pub fn exec(&mut self, instr: &Instruction) -> Result<()> {
        match instr {
            Instruction::Store => {
                let key = self.stack.pop();
                let value = self.stack.pop();

                self.contract_state.put(key.to_bytes(), value.to_bytes());
            }

            Instruction::Pack => {
                let n: usize = self.stack.pop().try_into()?;
                let item = if n <= 4 {
                    StackItem::Bytes4(self.get_bytes(n)?)
                } else if n <= 8 {
                    StackItem::Bytes8(self.get_bytes(n)?)
                } else if n <= 16 {
                    StackItem::Bytes16(self.get_bytes(n)?)
                } else if n <= 32 {
                    StackItem::Bytes32(self.get_bytes(n)?)
                } else if n <= 64 {
                    StackItem::Bytes64(self.get_bytes(n)?)
                } else {
                    return Err(anyhow!(
                        "can't put more than 64 bytes into byte array on vm stack"
                    ));
                };
                self.stack.push(item)
            }

            // TODO: change vm data insturction array to accept int
            Instruction::PushInt => {
                let i = self.ip.saturating_sub(1);
                self.stack.push(StackItem::Int(self.data[i] as i32));
            }
            Instruction::PushByte => {
                let i = self.ip.saturating_sub(1);
                self.stack.push(StackItem::Byte(self.data[i]));
            }
            Instruction::Add => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                let c = a.add(b)?;
                self.stack.push(c)
            }
            Instruction::Sub => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                let c = a.sub(b)?;
                self.stack.push(c)
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm() -> Result<()> {
        let mut state = State::new();
        let mut vm = VM::new(vec![0x03, 0x0a, 0x02, 0x0a, 0x0e], &mut state);
        vm.run()?;

        assert_eq!(StackItem::Int(1), vm.stack.pop());

        Ok(())
    }

    #[test]
    fn test_vm_pack() -> Result<()> {
        let data = vec![0x03, 0x0a, 0x46, 0x0c, 0x4f, 0x0c, 0x4f, 0x0c, 0x0d];

        let mut state = State::new();
        let mut vm = VM::new(data, &mut state);
        vm.run()?;

        println!("{:?}", vm.stack);

        let result = vm.stack.pop();
        assert_eq!("FOO", result.to_string()?);

        Ok(())
    }

    #[test]
    fn test_vm_store() -> Result<()> {
        let data = vec![
            0x03, 0x0a, 0x46, 0x0c, 0x4f, 0x0c, 0x4f, 0x0c, 0x0d, 0x05, 0x0a, 0x0f,
        ];

        let mut state = State::new();
        let mut vm = VM::new(data, &mut state);
        vm.run()?;

        assert_eq!(state.get(&vec![70, 79, 79, 0])?, vec![5]);

        println!("{:?}", state);

        Ok(())
    }
}
