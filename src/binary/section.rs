use num_derive::FromPrimitive;
use super::{instruction::Instruction, types::FunctionLocal};

// each section code is a u32
#[derive(Debug, PartialEq, Eq, FromPrimitive)]
pub enum SectionCode {
    Type = 0x01,
    Import = 0x02,
    Function = 0x03,
    Memory = 0x05,
    Export = 0x07,
    Code = 0x0a,
    Data = 0x0b,
}


// The Default trait provides a way to create a default value for a type. 
// For Function, it allows you to create an instance where locals and code 
// are empty vectors (i.e., Function { locals: vec![], code: vec![] }). 
// This is useful for initialization without specifying all fields manually.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub locals: Vec<FunctionLocal>,
    pub code: Vec<Instruction>,
}