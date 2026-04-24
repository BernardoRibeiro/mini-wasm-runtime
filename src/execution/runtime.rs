use super::{store::Store, value::Value};
use crate::binary::types::ValueType;
use crate::binary::{instruction::Instruction, module::Module};
use crate::execution::store::{FuncInst, InternalFuncInst};
use anyhow::Result;
use anyhow::bail;


#[derive(Default)]
pub struct Frame {
    pub pc: isize,               // Program counter
    pub sp: usize,               // Stack pointer
    pub insts: Vec<Instruction>, // Instructions
    pub arity: usize,            // Number of return values
    pub locals: Vec<Value>,      // Local variables
}

#[derive(Default)]
pub struct Runtime {
    pub store: Store,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    //wasm binary as argument and generates a Runtime
    pub fn instantiate(wasm: impl AsRef<[u8]>) -> Result<Self> {
        let module = Module::new(wasm.as_ref())?;
        let store = Store::new(module)?;
        Ok(Self {
            store,
            ..Default::default()
        })
    }

    fn execute(&mut self) -> Result<()> {
        loop {
            let Some(frame) = self.call_stack.last_mut() else {
                break;
            };

            frame.pc += 1;

            let Some(inst) = frame.insts.get(frame.pc as usize) else {
                break;
            };

            match inst {
                Instruction::I32Add => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack");
                    };
                    let result = left + right;
                    self.stack.push(result)
                }
                Instruction::LocalGet(idx) => {
                    //idx is a ref (&u32), *idx is to dereference
                    let Some(value) = frame.locals.get(*idx as usize) else {
                        bail!("not found local");
                    };
                    self.stack.push(*value)
                }
                Instruction::End => {
                    let Some(frame) = self.call_stack.pop() else {
                        bail!("not found frame");
                    };
                    let Frame { sp, arity, ..} = frame;
                    stack_unwind(&mut self.stack, sp, arity)?;
                }
            }
        }
        Ok(())
    }

    //invoke_internal for pre and post-processing of instruction execution

    // In Runtime::invoke_internal(...), the following steps are performed:

    // 1. Get the number of function arguments.
    // 2. Pop values from the stack for each argument.
    // 3. Initialize local variables.
    // 4. Get the number of function return values.
    // 5. Create a frame and push it onto Runtime::call_stack.
    // 6. Call Runtime::execute() to execute the function.
    // 7. If there are return values, pop them from the stack and return them; otherwise, return None.
    

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        let bottom = self.stack.len() - func.func_type.params.len();
        // `split_off(bottom)` removes all elements from self.stack from index `bottom` onwards,
        // and returns them as a new Vec. This is commonly used to separate the parameters (and
        // possibly return values) for a function call from the rest of the stack, leaving the
        // previous stack untouched below `bottom`.
        let mut locals = self.stack.split_off(bottom);

        for local in func.code.locals.iter() {
            match local {
                ValueType::I32 => locals.push(Value::I32(0)),
                ValueType::I64 => locals.push(Value::I64(0)),
            }
        }

        let arity = func.func_type.results.len();

        let frame = Frame {
            pc: -1,
            sp: self.stack.len(),
            insts: func.code.body.clone(),
            arity,
            locals,
        };

        self.call_stack.push(frame);

        if let Err(e) = self.execute() {
            self.cleanup();
            bail!("failed to execute instructions: {}" ,e);
        };

        if arity > 0 {
            let Some(value) = self.stack.pop() else {
                bail!("not found return values");
            };
            return Ok(Some(value));
        }
        Ok(None)
    }

    fn cleanup(&mut self) {
        self.stack = vec![];
        self.call_stack = vec![];
    }

    pub fn call(&mut self, idx: usize, args: Vec<Value>) -> Result<Option<Value>> {
        let Some(func_inst) = self.store.funcs.get(idx) else {
            bail!("not found func");
        };

        for arg in args {
            self.stack.push(arg);
        }

        match func_inst {
            FuncInst::Internal(func) => self.invoke_internal(func.clone()),
        }
    }
}

pub fn stack_unwind(stack: &mut Vec<Value>, sp: usize, arity: usize) -> Result<()> {
    if arity > 0 { // return value exists
        let Some(value) = stack.pop() else {
            bail!("not found return value");
        };
        stack.drain(sp..); // drain(sp..), which would remove (and return) all elements in the stack from index sp onward. This is used to discard local variables past the current function's stack pointer during stack unwinding.
        stack.push(value);
    }
    else {
        stack.drain(sp..);
    }
    Ok(())
}
