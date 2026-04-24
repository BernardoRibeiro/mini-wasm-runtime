use crate::binary::{
    instruction::Instruction, module::Module, types::{FuncType, ValueType}
};
use anyhow::{bail, Result};


#[derive(Clone)]
pub struct Func {
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

#[derive(Clone)]
pub struct InternalFuncInst {
    pub func_type: FuncType,
    pub code: Func,
}

#[derive(Clone)]
pub enum FuncInst {
    Internal(InternalFuncInst),
}

#[derive(Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
}

impl Store {
    pub fn new (module: Module) -> Result<Self> {
        //initialize func type indexes
        let func_type_idxs = match module.function_section {
            Some (ref idxs) => idxs.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];

        // The zip function combines two iterators into a single iterator
        //  of pairs, stopping when either iterator is exhausted. 
        //  Here, it pairs each func_body from code_section with the 
        //  corresponding type_idx from func_type_idxs, 
        //  allowing you to process them together in the loop.

        if let Some(ref code_section) = module.code_section {
            for (func_body, type_idx) in code_section.iter().zip(func_type_idxs.into_iter()){
                let Some(ref func_types) = module.type_section else {
                    bail!("not found type_section")
                };

                let Some(func_type) = func_types.get(type_idx as usize) else {
                    // `bail!` is a macro from the `anyhow` crate that immediately returns an error from the current function.
                    // It is equivalent to: return Err(anyhow::anyhow!("not found func type in type_section"));
                    bail!("not found func type in type_section")
         
                };

                // with_capacity creates a new Vec with enough pre-allocated space
                // for the given number of elements (func_body.locals.len()),
                // which avoids reallocations as items are added. 
                // This improves performance when the final size is known in advance.

                let mut locals = Vec::with_capacity(func_body.locals.len());
                for local in func_body.locals.iter() {
                    for _ in 0..local.type_count {
                        locals.push(local.value_type.clone());
                    }
                }

                let func = FuncInst::Internal(InternalFuncInst {
                    func_type: func_type.clone(),
                    code: Func {
                        locals,
                        body: func_body.code.clone(),
                    }
                });
                funcs.push(func)
            }
        }
        Ok(Self {funcs})
    }
}