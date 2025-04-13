use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::string::ToString;
use crate::building::{Building, ProcessorBuilding};
use crate::instruction::Instruction;
use crate::value::Value;
use crate::variable::{VarHandle, Variable, Variables};

#[derive(Debug)]
pub enum VmError {
    InvalidCast(String, &'static str, &'static str),
    InvalidBuildingType(&'static str, String),
    VariableNotFound(String),
    ConstantMutation(String),
    EmptyCode,
    CodeTooLong(usize, usize),
}

#[derive(Debug)]
pub struct PosVmError(VmError, Option<usize>);

pub type VmResult<T> = Result<T, VmError>;
pub type PosVmResult<T> = Result<T, PosVmError>;

impl VmError {
    pub fn to_pos(self) -> PosVmError {
        PosVmError(self, None)
    }

    pub fn with_pos(self, pos: usize) -> PosVmError {
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
            VmError::CodeTooLong(len, limit) =>
                write!(f, "Program has too many instructions ({} > {})", len, limit),
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
    handle_counter: VarHandle,
    handle_links: VarHandle,
    variables: Rc<RefCell<Variables>>,
    code: Vec<Instruction>,
    print_buffer: String,
}

macro_rules! builtin {
    ($name:expr, $val:expr) => {
        {
            ($name, Variable::new_const(Rc::new($name.to_string()), $val, true))
        }
    };
    ($name:expr, $val:expr, $constant:expr) => {
        {
            ($name, Variable::new_const(Rc::new($name.to_string()), $val, $constant))
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

    pub fn new(code: &str, code_len_limit: usize) -> VmResult<Self> {
        let mut vars = Variables::from([
            builtin!("@counter", num!(), false),
            builtin!("@thisx", num!()),
            builtin!("@thisy", num!()),
            builtin!("@ipt", num!(1000.)),
            builtin!("@timescale", num!(1.)),
            builtin!("@links", num!()),
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
        let code = code.split("\n").filter_map(|ln| Instruction::parse(ln, &mut vars)).collect::<Vec<_>>();
        if code.is_empty() {
            return Err(VmError::EmptyCode);
        }
        if code.len() > code_len_limit {
            return Err(VmError::CodeTooLong(code.len(), code_len_limit));
        }
        let vm = VM {
            handle_counter: vars.get_handle("@counter").unwrap(),
            handle_links: vars.get_handle("@links").unwrap(),
            variables: Rc::new(RefCell::new(vars)),
            code,
            print_buffer: "".to_string(),
        };
        let this_handle = vm.variables.borrow_mut().handle("@this");
        this_handle.set(&mut vm.variables.borrow_mut(), Value::Building(Rc::new(
            ProcessorBuilding::new("@this".to_string(), Rc::downgrade(&vm.variables)))))?;
        Ok(vm)
    }

    pub fn get_val(&self, name: &str) -> VmResult<Value> {
        let vars = self.variables.borrow();
        vars.get_handle(name)
            .ok_or_else(|| VmError::VariableNotFound(name.to_string()))
            .map(|h| h.val(&vars).clone())
    }

    pub fn building<T: Building + 'static>(&mut self, building: T) -> VmResult<Rc<T>> {
        let mut vars = self.variables.borrow_mut();
        let new_val = num!(self.handle_counter.get(&vars).as_num()? + 1.);
        self.handle_counter.set(&mut vars, new_val)?;
        let building = Rc::new(building);
        vars.handle(building.name())
            .set(&mut vars, Value::Building(building.clone()))
            .map(|_| building)
    }

    pub fn cycle(&mut self) -> PosVmResult<()> {
        let mut vars = self.variables.borrow_mut();
        let pc = match self.handle_counter.get(&vars).as_num() {
            Ok(pc) => pc,
            Err(err) => return Err(err.to_pos()),
        } as i64;
        let pc: usize = if pc < 0 || pc >= self.code.len() as i64 {
            0
        } else {
            pc as usize
        };
        self.handle_counter.set(&mut vars, num!(pc as f64 + 1.)).unwrap();
        match self.code[pc].execute(&mut vars, &mut self.print_buffer) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.with_pos(pc)),
        }
    }
}
