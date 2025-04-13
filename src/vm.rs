use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::string::ToString;
use crate::building::{Building, ProcessorBuilding};
use crate::instruction::Instruction;
use crate::value::Value;
use crate::variable::{Variable, Variables};

const CODE_LEN_LIMIT: usize = 1000;

#[derive(Debug)]
pub enum VmError {
    InvalidCast(String, &'static str, &'static str),
    InvalidBuildingType(&'static str, String),
    VariableNotFound(String),
    ConstantMutation(String),
    EmptyCode,
    CodeTooLong(usize),
}

#[derive(Debug)]
pub struct PosVmError(VmError, Option<u16>);

pub type VmResult<T> = Result<T, VmError>;
pub type PosVmResult<T> = Result<T, PosVmError>;

impl VmError {
    pub fn to_pos(self) -> PosVmError {
        PosVmError(self, None)
    }

    pub fn with_pos(self, pos: u16) -> PosVmError {
        PosVmError(self, Some(pos))
    }

    fn print(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::InvalidCast(value, from, to) =>
                write!(f, "Cannot cast value '{}' of type '{}' to type '{}'", value, from, to),
            VmError::InvalidBuildingType(action, name) =>
                write!(f, "Cannot {} building '{}'", action, name),
            VmError::VariableNotFound(name) => write!(f, "Variable not found: '{}'", name),
            VmError::ConstantMutation(name) =>
                write!(f, "Cannot mutate constant variable '{}'", name),
            VmError::EmptyCode =>
                write!(f, "Program is empty"),
            VmError::CodeTooLong(len) =>
                write!(f, "Program has too many instructions ({} > {})", len, CODE_LEN_LIMIT),
        }
    }
}

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: ")?;
        self.print(f)
    }
}

impl Display for PosVmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let PosVmError(err, pos) = self;
        if let Some(pos) = pos {
            write!(f, "Error at instruction {}: ", pos)?;
            err.print(f)
        } else {
            write!(f, "{}", err)
        }
    }
}

#[derive(Debug)]
pub struct VM {
    var_counter: String,
    variables: Rc<RefCell<Variables>>,
    code: Vec<Instruction>,
    print_buffer: String,
}

macro_rules! builtin {
    ($name:expr, $val:expr) => {
        {
            let name = Rc::new($name.to_string());
            (name.clone(), Variable::new(name, $val))
        }
    };
    ($name:expr, $val:expr, $constant:expr) => {
        {
            let name = Rc::new($name.to_string());
            (name.clone(), Variable::new_const(name, $val, $constant))
        }
    }
}

macro_rules! num {
    ($val:expr) => (Value::Num($val));
}

impl VM {
    pub fn new(code: &str) -> VmResult<Self> {
        let code = code.split("\n").filter_map(Instruction::parse).collect::<Vec<_>>();
        if code.is_empty() {
            return Err(VmError::EmptyCode);
        }
        if code.len() > CODE_LEN_LIMIT {
            return Err(VmError::CodeTooLong(code.len()));
        }
        let vm = VM {
            var_counter: "@counter".to_string(),
            variables: Rc::new(RefCell::new(Variables::from([
                builtin!("@counter", num!(0.)),
                builtin!("null", Value::Null, true),
                builtin!("true", num!(1.), true),
                builtin!("false", num!(0.), true),
            ]))),
            code,
            print_buffer: "".to_string(),
        };
        vm.variables.borrow_mut().insert(
            "@this", Value::Building(Rc::new(
                ProcessorBuilding::new("@this".to_string(), Rc::downgrade(&vm.variables)))))?;
        Ok(vm)
    }

    pub fn get_val(&self, name: &str) -> VmResult<Value> {
        self.variables.borrow().get(&name.to_string()).map(|var| var.val().clone())
    }

    pub fn building<T: Building + 'static>(&mut self, building: T) -> VmResult<Rc<T>> {
        let building = Rc::new(building);
        self.variables.borrow_mut().set(Rc::new(building.name().to_string()),
                                        Value::Building(building.clone())).map(|_| building)
    }

    pub fn cycle(&mut self) -> PosVmResult<()> {
        let pc = match self.variables.borrow().get(&self.var_counter) {
            Ok(pc) => match pc.as_num() {
                Ok(pc) => pc,
                Err(err) => return Err(err.to_pos()),
            },
            Err(err) => return Err(err.to_pos()),
        } as i64;
        let pc: u16 = if pc < 0 || pc >= self.code.len() as i64 {
            0
        } else {
            pc as u16
        };
        self.variables.borrow_mut().set(Rc::new(self.var_counter.clone()),
                                        num!(pc as f64 + 1.)).unwrap();
        match self.code[pc as usize].execute(&mut *self.variables.borrow_mut(),
                                             &mut self.print_buffer) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.with_pos(pc)),
        }
    }
}
