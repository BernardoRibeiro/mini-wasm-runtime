// we use super since section.rs is in the same file as module.rs
use super::{section::SectionCode, types::FuncType};
use crate::binary::{instruction::Instruction, opcode::Opcode, section::Function, types::{FunctionLocal, ValueType}};

use nom::{
    IResult, bytes::complete::{tag, take}, multi::many0, number::complete::{le_u8, le_u32}, sequence::pair
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

    fn decode_value_type(input: &[u8]) -> IResult<&[u8], ValueType> {
        let (input, value_type) = le_u8(input)?;
        Ok((input, value_type.into()))
    }

    // Type section example
        //     ; section "Type" (1)
        // 0000008: 01       ; section code
        // 0000009: 0d       ; section size
        // 000000a: 02       ; num types
        // ; func type 0
        // 000000b: 60       ; func       
        // 000000c: 02       ; num params 
        // 000000d: 7f       ; i32        
        // 000000e: 7e       ; i64        
        // 000000f: 00       ; num results
        // ; func type 1
        // 0000010: 60       ; func       
        // 0000011: 02       ; num params 
        // 0000012: 7e       ; i64        
        // 0000013: 7f       ; i32        
        // 0000014: 02       ; num results
        // 0000015: 7f       ; i32        
        // 0000016: 7e       ; i64        


    fn decode_type_section(input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
        let mut func_types: Vec<FuncType> = vec![];

        let (mut input, count) = leb128_u32(input)?; // 1 (num types)

        for _ in 0..count {
            let (rest, _) = le_u8(input)?;// 2 (func)

            let mut func = FuncType::default();

            let (rest, size) = leb128_u32(rest)?; // 3 (num params)
            let (rest, types) = take(size)(rest)?;
            let (_, types) = many0(Self::decode_value_type)(types)?; // 4 
            func.params = types;

            let (rest, size) = leb128_u32(rest)?; // 5 (num resutls)
            let (rest, types) = take(size)(rest)?;
            let (_, types) = many0(Self::decode_value_type)(types)?; // 6
            func.results = types;

            func_types.push(func);
            input = rest;
        }
        Ok((&[], func_types))
    }

//     ; section "Code" (10)
    // 0000012: 0a         ; section code
    // 0000013: 08         ; section size 
    // 0000014: 01         ; num functions
    // ; function body 0
    // 0000015: 06         ; func body size 
    // 0000016: 02         ; local decl count
    // 0000017: 01         ; local type count
    // 0000018: 7f         ; i32
    // 0000019: 02         ; local type count 2 means two local variables with this type
    // 000001a: 7e         ; i64
    // 000001b: 0b         ; end


    fn decode_code_section(input: &[u8]) -> IResult<&[u8], Vec<Function>> {
        let mut functions = vec![];
        let (mut input, count) = leb128_u32(input)?; // 1 (num functions)

        for _ in 0..count {
            let (rest, size) = leb128_u32(input)?; // 2 (body size)
            let (rest, body) = take(size)(rest)?; // 3 ()
            let (_, body) = Self::decode_function_body(body)?; // 4 (body)
            functions.push(body);
            input = rest;
        }

        Ok((&[], functions))
    }

    fn decode_function_body(input: &[u8]) -> IResult<&[u8], Function> {
        let mut body = Function::default();

        let (mut input, count) = leb128_u32(input)?; // 4-1
        
        for _ in 0..count { // 4-2
            let (rest, type_count) = leb128_u32(input)?; // 4-3
            let (rest, value_type) = le_u8(rest)?; // 4-4
            body.locals.push(FunctionLocal {
                    type_count,
                    value_type: value_type.into(),
            });
            input = rest;
        }
                
        let mut remaining = input;
        while !remaining.is_empty() { // 5
            let (rest, inst) = Self::decode_instructions(remaining)?;
            body.code.push(inst);
            remaining = rest;
        }
             
        Ok((&[], body))
    }

     
    fn decode_instructions(input: &[u8]) -> IResult<&[u8], Instruction> {
        let (input, byte) = le_u8(input)?;
        let op = Opcode::from_u8(byte).unwrap_or_else(|| panic!("invalid opcode: {:X}", byte)); // 5-1
        let (rest, inst) = match op { // 5-2
            Opcode::LocalGet => { // 5-2-1
                let (rest, idx) = leb128_u32(input)?;
                (rest, Instruction::LocalGet(idx))
            }
            Opcode::I32Add => (input, Instruction::I32Add), // 5-2-2
            Opcode::End => (input, Instruction::End), // 5-2-2
        };
        Ok((rest, inst))
    }
    

}
#[cfg(test)]
mod tests {
    use crate::binary::module::Module;
    use crate::binary::types::{FunctionLocal, ValueType};
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

    #[test]
    fn decode_func_params() -> Result<()> {
        let wasm = wat::parse_str("(module (func (param i32 i64)))")?;
        let module = Module::new(&wasm)?;

        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I64],
                    results: vec![],
                }]),
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

    #[test]
    fn decode_func_local() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_local.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType::default()]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![
                        FunctionLocal {
                            type_count: 1,
                            value_type: ValueType::I32,
                        },
                        FunctionLocal {
                            type_count: 2,
                            value_type: ValueType::I64,
                        },
                    ],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    
    #[test]
    fn decode_func_add() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::LocalGet(1),
                        Instruction::I32Add,
                        Instruction::End
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }
        
}
