use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::string::ToString;
use serde::Serialize;
use crate::building::{Building, ProcessorBuilding};
use crate::instruction::Instruction;
use crate::value::{Property, Value};
use crate::variable::{VarHandle, Variable, Variables};

#[derive(Debug)]
pub enum VmError {
    InvalidCast(String, &'static str, &'static str),
    InvalidBuildingType(&'static str, String),
    VariableNotFound(String),
    ConstantMutation(String),
    EmptyCode,
    CodeTooLong(usize, usize),
    InvalidCharacter(u16),
    NegativeIndex(i64, &'static str),
    IndexTooHigh(usize, usize, &'static str),
    PcResError(Box<VmError>),
    InvalidFormat(String),
    NoProperty(String, &'static str, &'static str),
    InvalidOperation(String),
    DivisionByZero,
}

#[derive(Debug)]
pub struct PosVmError(pub VmError, pub Option<usize>);

pub type VmResult<T> = Result<T, VmError>;
pub type PosVmResult<T> = Result<T, PosVmError>;

impl VmError {
    pub fn to_pos(self) -> PosVmError {
        PosVmError(self, None)
    }

    pub fn with_pos(self, pos: usize) -> PosVmError {
        PosVmError(self, Some(pos))
    }

    pub fn to_pc_res(self) -> VmError {
        VmError::PcResError(Box::new(self))
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
            VmError::CodeTooLong(len, limit) =>
                write!(f, "Program has too many instructions ({} > {})", len, limit),
            VmError::InvalidCharacter(ch) =>
                write!(f, "Invalid UTF-16 character: {}", ch),
            VmError::NegativeIndex(idx, device) =>
                write!(f, "Negative index ({}) for {}", idx, device),
            VmError::IndexTooHigh(idx, limit, device) =>
                write!(f, "Index out of range ({} >= {}) for {}", idx, limit, device),
            VmError::PcResError(err) =>
                err.print(f),
            VmError::InvalidFormat(msg) =>
                write!(f, "Invalid format - {}", msg),
            VmError::NoProperty(value, type_, prop) =>
                write!(f, "Value '{}' of type '{}' has no property '{}'", value, type_, prop),
            VmError::InvalidOperation(op) =>
                write!(f, "Invalid operation: '{}'", op),
            VmError::DivisionByZero =>
                write!(f, "Division by zero"),
        }
    }
}

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::PcResError(_) => write!(f, "Error during program counter resolution: ")?,
            _ => write!(f, "Error: ")?,
        }
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
pub struct VmCycleResult {
    pub pc_wrap: bool,
    pub halt: bool,
}

#[derive(Debug, Serialize)]
pub enum VmFinishReason {
    PcWrap,
    Halt,
    InsLimit,
}

#[derive(Debug)]
pub struct PrintBuffer {
    string: RefCell<String>,
}

impl PrintBuffer {
    pub fn new() -> Self {
        PrintBuffer {
            string: RefCell::new("".to_string())
        }
    }

    pub fn write(&self, string: &str) {
        self.string.borrow_mut().push_str(string);
    }

    pub fn write_utf_16(&self, ch: u16) -> VmResult<()> {
        self.write(&String::from_utf16(&[ch]).map_err(|_| VmError::InvalidCharacter(ch))?);
        Ok(())
    }

    pub fn format(&self, _string: &str) -> VmResult<()> {
        Err(VmError::InvalidFormat("not implemented".to_string()))
    }

    pub fn take(&self) -> String {
        self.string.replace("".to_string())
    }
}

#[derive(Debug)]
pub struct VM {
    pc_handle: VarHandle,
    variables: Rc<Variables>,
    code: Vec<Instruction>,
    print_buffer: PrintBuffer,
    buildings: Vec<Rc<dyn Building>>,
}

macro_rules! builtin {
    ($name:expr, $val:expr) => {
        {
            ($name, Variable::new_const($name.to_string(), $val, true))
        }
    };
    ($name:expr, $val:expr, $constant:expr) => {
        {
            ($name, Variable::new_const($name.to_string(), $val, $constant))
        }
    }
}

macro_rules! num {
    () => (Value::Num(0.));
    ($val:expr) => (Value::Num($val));
}

macro_rules! null {
    () => (Value::Null);
}

impl VM {
    pub const DEFAULT_CODE_LEN_LIMIT: usize = 1000;

    pub fn new(code: &str, code_len_limit: usize, buildings: Vec<Rc<dyn Building>>) -> VmResult<Self> {
        let mut vars = Variables::from([
            builtin!("@counter", num!(), false),
            builtin!("@this", null!()),
            builtin!("@thisx", num!()),
            builtin!("@thisy", num!()),
            builtin!("@ipt", num!(1000.)),
            builtin!("@timescale", num!(1.)),
            builtin!("@links", num!(buildings.len() as f64)),
            builtin!("@unit", null!(), false),
            builtin!("@time", num!()),
            builtin!("@tick", num!()),
            builtin!("@second", num!()),
            builtin!("@minute", num!()),
            builtin!("@waveNumber", num!()),
            builtin!("@waveTime", num!()),
            builtin!("@mapw", num!()),
            builtin!("@maph", num!()),
            builtin!("null", null!()),
            builtin!("true", num!(1.)),
            builtin!("false", num!(0.)),
            builtin!("@pi", num!(std::f64::consts::PI)),
            builtin!("@e", num!(std::f64::consts::E)),
            builtin!("@degToRad", num!(std::f64::consts::PI / 180.)),
            builtin!("@radToDeg", num!(180. / std::f64::consts::PI)),
            builtin!("blockCount", num!()),
            builtin!("unitCount", num!()),
            builtin!("itemCount", num!()),
            builtin!("liquidCount", num!()),
        ]);
        for name in Property::PROPERTIES {
            let var_name = "@".to_string() + name;
            vars.insert(var_name.clone(), Variable::new_const(
                var_name, Value::Property(Property::new(name)), true));
        }
        for building in &buildings {
            vars.insert(building.name().to_string(),
                        Variable::new_const(building.name().to_string(),
                                            Value::Building(building.clone()), true));
        }
        let code = code.split("\n")
            .filter_map(|ln| Instruction::parse(ln, &mut vars)).collect::<Vec<_>>();
        if code.is_empty() {
            return Err(VmError::EmptyCode);
        }
        if code.len() > code_len_limit {
            return Err(VmError::CodeTooLong(code.len(), code_len_limit));
        }
        let vm = VM {
            pc_handle: vars.get_handle("@counter").unwrap(),
            variables: Rc::new(vars),
            code,
            print_buffer: PrintBuffer::new(),
            buildings,
        };
        vm.variables.get_handle("@this").unwrap().force_set(&vm.variables, Value::Building(
            Rc::new(ProcessorBuilding::new("@this".to_string(), Rc::downgrade(&vm.variables)))));
        Ok(vm)
    }

    pub fn get_val(&self, name: &str) -> VmResult<Value> {
        self.variables.get_handle(name)
            .ok_or_else(|| VmError::VariableNotFound(name.to_string()))
            .map(|h| h.val(&self.variables).clone())
    }

    pub fn cycle(&self) -> PosVmResult<VmCycleResult> {
        let pc = match self.pc_handle.get(&self.variables).as_int() {
            Ok(pc) => pc,
            Err(err) => return Err(err.to_pc_res().to_pos()),
        };
        if pc < 0 {
            return Err(VmError::NegativeIndex(pc, "program counter").to_pos());
        }
        let (pc, pc_wrap): (usize, bool) = if pc >= self.code.len() as i64 {
            (0, true)
        } else {
            (pc as usize, false)
        };
        self.pc_handle.set(&self.variables, num!(pc as f64 + 1.)).unwrap();
        match self.code[pc].execute(&self.variables, &self.print_buffer,
                                    &self.buildings, self.pc_handle) {
            Ok(res) => Ok(VmCycleResult {
                pc_wrap,
                halt: res.halt,
            }),
            Err(err) => Err(err.with_pos(pc)),
        }
    }

    pub fn run(&self, limit: Option<usize>, end_on_wrap: bool) -> PosVmResult<VmFinishReason> {
        for _ in 0..limit.unwrap_or(usize::MAX) {
            let res = self.cycle()?;
            if res.halt {
                return Ok(VmFinishReason::Halt);
            } else if res.pc_wrap && end_on_wrap {
                return Ok(VmFinishReason::PcWrap);
            }
        }
        Ok(VmFinishReason::InsLimit)
    }

    pub fn into_print_buffer(self) -> PrintBuffer {
        self.print_buffer
    }
}
