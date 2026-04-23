// we use super since section.rs is in the same file as module.rs
use super::{section::SectionCode, types::FuncType};
use crate::binary::{instruction::Instruction, section::Function};

use nom::{
    bytes::complete::{tag, take},
    number::complete::{le_u32, le_u8},
    sequence::pair,
    IResult,
};
use nom_leb128::leb128_u32;
use num_traits::FromPrimitive as _;

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub type_section: Option<Vec<FuncType>>,
    pub function_section: Option<Vec<u32>>,
    pub code_section: Option<Vec<Function>>,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            magic: "\0asm".to_string(),
            version: 1,
            type_section: None,
            function_section: None,
            code_section: None,
        }
    }
}

impl Module {
    pub fn new(input: &[u8]) -> anyhow::Result<Module> {
        let (_, module) =
            Module::decode(input).map_err(|e| anyhow::anyhow!("failed to parse wasm: {}", e))?;
        Ok(module)
    }

    fn decode(input: &[u8]) -> IResult<&[u8], Module> {
        // The output is a tuple: the remaining input (after \0asm is consumed) 
        // and the matched value (here, b"\0asm").
        let (input, _) = tag(b"\0asm")(input)?;
        // In the Wasm spec, binaries are encoded in little-endian.
        let (input, version) = le_u32(input)?;

        let mut module = Module {
            magic: "\0asm".to_string(),
            version,
            ..Default::default()
        };

        // decode the sections
        let mut remaining = input;
        while !remaining.is_empty() {
            let (section_input, (code, size)) = Self::decode_section_header(remaining)?;
            let (rest, section_contents) = take(size)(section_input)?;

            match code {
                SectionCode::Type => {
                    let (_, types) = Self::decode_type_section(section_contents)?;
                    module.type_section = Some(types);
                }
                SectionCode::Function => {
                    let (_, func_idx_list) = Self::decode_function_section(section_contents)?;
                    module.function_section = Some(func_idx_list);
                }
                SectionCode::Code => {
                    let (_, functions) = Self::decode_code_section(section_contents)?;
                    module.code_section = Some(functions);
                }
                _ => todo!(),
            }

            remaining = rest;
        }
        Ok((input, module))
    }

    fn decode_section_header(input: &[u8]) -> IResult<&[u8], (SectionCode, u32)> {
        let (input, (code, size)) = pair(le_u8, leb128_u32)(input)?; // 1
        Ok((
            input,
            (
                SectionCode::from_u8(code).expect("unexpected section code"), // 2
                size,
            )
        ))
    }

    fn decode_type_section(_input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
        let func_types = vec![FuncType::default()];
        // TODO: Decoding arguments and return values

        Ok((&[], func_types))

    }

    fn decode_function_section(input: &[u8]) -> IResult<&[u8], Vec<u32>> {
        let mut func_idx_list: Vec<u32> = vec![];
        let (mut input, func_count) = leb128_u32(input)?;

        for _ in 0..func_count {
            let (rest, func_idx) = leb128_u32(input)?;
            func_idx_list.push(func_idx);
            input = rest;
        }
        Ok((&[], func_idx_list))
    }

    fn decode_code_section(input: &[u8]) -> IResult<&[u8], Vec<Function>> {
        // TOOD: Decoding local variables and instructions
        let functions = vec![Function {
            locals: vec![],
            code: vec![Instruction::End],
        }];
        Ok((&[], functions))

    }
}
#[cfg(test)]
mod tests {
    use crate::binary::module::Module;
    use crate::binary::{instruction::Instruction, section::Function, types::FuncType};
    use anyhow::Result;

    #[test]
    fn decode_simplest_module() -> Result<()> {
        // Generate wasm binary with only preamble present
        let wasm = wat::parse_str("(module)")?;
        // Decode binary and generate Module structure
        let module = Module::new(&wasm)?;
        // Compare whether the generated Module structure is as expected
        assert_eq!(module, Module::default());
        Ok(())
    }

    #[test]
    fn decode_simple_function() -> Result<()> {
        let wasm = wat::parse_str("(module (func))")?;
        let module = Module::new(&wasm)?;

        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType::default()]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }
}
