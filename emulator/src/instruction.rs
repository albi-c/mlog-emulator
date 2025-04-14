use std::random::random;
use std::rc::Rc;
use std::str::FromStr;
use strum_macros::EnumString;
use crate::building::Building;
use crate::value::Value;
use crate::variable::{VarHandle, Variables};
use crate::vm::{PrintBuffer, VmError, VmResult};

#[derive(Debug)]
pub enum ValueArg {
    Value(Value),
    Variable(VarHandle),
}

impl ValueArg {
    fn parse(string: &str, vars: &mut Variables) -> Self {
        if string.starts_with("\"") && string.ends_with("\"") {
            ValueArg::Value(Value::Str(Rc::new(string[1..string.len()-1].into())))
        } else if let Ok(num) = f64::from_str(string) {
            ValueArg::Value(Value::Num(num))
        } else {
            ValueArg::Variable(vars.handle(string))
        }
    }

    pub fn eval(&self, vars: &Variables) -> VmResult<Value> {
        Ok(match self {
            ValueArg::Value(val) => val.clone(),
            ValueArg::Variable(var) => var.val(vars),
        })
    }
}

#[derive(Debug)]
pub struct InstructionExecuteResult {
    pub halt: bool,
}

#[derive(Debug, EnumString)]
#[strum(serialize_all = "camelCase")]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Idiv,
    Mod,
    Pow,
    Not,
    Land,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    StrictEqual,
    Equal,
    NotEqual,
    Shl,
    Shr,
    Or,
    And,
    Xor,
    Flip,
    Max,
    Min,
    Abs,
    Log,
    Log10,
    Floor,
    Ceil,
    Sqrt,
    Angle,
    Length,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Rand,
}

#[derive(Debug)]
pub enum Instruction {
    Read(VarHandle, ValueArg, ValueArg),
    Write(ValueArg, ValueArg, ValueArg),
    Print(ValueArg),
    PrintChar(ValueArg),
    Format(ValueArg),

    PrintFlush(ValueArg),
    GetLink(VarHandle, ValueArg),
    Sensor(VarHandle, ValueArg, ValueArg),

    Set(VarHandle, ValueArg),
    Op(Operator, VarHandle, ValueArg, ValueArg),

    Wait(ValueArg),
    Stop,
    End,
    Jump(ValueArg, String, ValueArg, ValueArg),
}

macro_rules! arg {
    (out, $vars:expr, $arg:expr) => ($vars.handle($arg));
    (in, $vars:expr, $arg:expr) => (ValueArg::parse($arg, $vars));
    (imm, $vars:expr, $arg:expr) => (String::from($arg));
    (op, $vars:expr, $arg:expr) => (Operator::from_str($arg).unwrap());
}

macro_rules! ins {
    ($ins:ident, $vars:expr, $args:expr) => {
        Instruction::$ins
    };
    ($ins:ident, $vars:expr, $args:expr => $($sel:tt $i:expr),*) => {
        Instruction::$ins($(arg!($sel, $vars, $args[$i])),*)
    };
}

macro_rules! two_nums {
    ($vars:ident, $a:ident, $b:ident) => {
        ($a.eval($vars)?.as_num()?, $b.eval($vars)?.as_num()?)
    };
}

macro_rules! binary {
    ($vars:ident, $a:ident, $b:ident, fn $func:expr) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num($func(a, b))
        }
    };
    ($vars:ident, $a:ident, $b:ident, ?fn $func:expr) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num($func(a, b)?)
        }
    };
    ($vars:ident, $a:ident, $b:ident, $op:tt) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num(a $op b)
        }
    };
    ($vars:ident, $a:ident, $b:ident, !fn $func:expr) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num(if $func(a, b) { 1. } else { 0. })
        }
    }
}

macro_rules! binary_i {
    ($vars:ident, $a:ident, $b:ident, fn $func:expr) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num($func(a as i64, b as i64) as f64)
        }
    };
    ($vars:ident, $a:ident, $b:ident, ?fn $func:expr) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num($func(a as i64, b as i64)? as f64)
        }
    };
    ($vars:ident, $a:ident, $b:ident, $op:tt) => {
        {
            let (a, b) = two_nums!($vars, $a, $b);
            Value::Num(((a as i64) $op (b as i64)) as f64)
        }
    };
}

macro_rules! unary {
    ($vars:ident, $a:ident, fn $func:expr) => {
        {
            Value::Num($func($a.eval($vars)?.as_num()?))
        }
    };
    ($vars:ident, $a:ident, $op:tt) => {
        {
            Value::Num($op $a.eval($vars)?.as_num()?)
        }
    };
}

impl Instruction {
    fn split_line(line: &str) -> Vec<&str> {
        let mut segments = vec![];
        let mut start = 0;
        let mut next_start = false;
        let mut last_ch = 0;
        let mut quotes = false;
        for (i, ch) in line.char_indices() {
            if ch == ' ' && !quotes {
                segments.push(&line[start..i]);
                next_start = true;
            } else if next_start {
                next_start = false;
                start = i;
            }
            if ch == '"' {
                quotes = !quotes;
            }
            last_ch = i + ch.len_utf8();
        }
        if start != last_ch {
            segments.push(&line[start..last_ch]);
        }
        segments
    }

    pub fn parse(line: &str, vars: &mut Variables) -> Option<Self> {
        let args = Self::split_line(line);
        if args.is_empty() {
            return None;
        }
        Some(match args[0] {
            "read" => ins!(Read, vars, args => out 1, in 2, in 3),
            "write" => ins!(Write, vars, args => in 1, in 2, in 3),
            "print" => ins!(Print, vars, args => in 1),
            "printchar" => ins!(PrintChar, vars, args => in 1),
            "format" => ins!(Format, vars, args => in 1),

            "printflush" => ins!(PrintFlush, vars, args => in 1),
            "getlink" => ins!(GetLink, vars, args => out 1, in 2),
            "sensor" => ins!(Sensor, vars, args => out 1, in 2, in 3),

            "set" => ins!(Set, vars, args => out 1, in 2),
            "op" => ins!(Op, vars, args => op 1, out 2, in 3, in 4),

            "wait" => ins!(Wait, vars, args => in 1),
            "stop" => ins!(Stop, vars, args),
            "end" => ins!(End, vars, args),
            "jump" => ins!(Jump, vars, args => in 1, imm 2, in 3, in 4),

            name => panic!("Unsupported instruction: '{}'", name),
        })
    }

    pub fn execute(&self, vars: &Variables, print_buffer: &PrintBuffer,
                   buildings: &[Rc<dyn Building>], pc: VarHandle) -> VmResult<InstructionExecuteResult> {
        match self {
            Instruction::Read(dst, src, idx) => {
                let src = src.eval(vars)?;
                let idx = idx.eval(vars)?;
                dst.set(vars, if let Ok(string) = src.as_str() {
                    Value::Num(idx.do_index_copy(string.as_utf_16(), "string")? as f64)
                } else {
                    src.as_building()?.read(idx)?
                })?
            },
            Instruction::Write(src, dst, idx) =>
                dst.eval(vars)?.as_building()?.write(idx.eval(vars)?, src.eval(vars)?)?,
            Instruction::Print(val) =>
                print_buffer.write(&val.eval(vars)?.to_string()),
            Instruction::PrintChar(val) =>
                print_buffer.write_utf_16(val.eval(vars)?.as_int()? as u16)?,
            Instruction::Format(val) =>
                print_buffer.format(&val.eval(vars)?.to_string())?,

            Instruction::PrintFlush(val) =>
                val.eval(vars)?.as_building()?.print_flush(print_buffer.take())?,
            Instruction::GetLink(dst, idx) =>
                dst.set(vars, Value::Building(
                    idx.eval(vars)?.do_index(buildings, "get link")?.clone()))?,
            Instruction::Sensor(dst, src, prop) =>
                dst.set(vars, src.eval(vars)?.sense(prop.eval(vars)?.as_property()?)?)?,

            Instruction::Set(dst, src) =>
                dst.set(vars, src.eval(vars)?)?,
            Instruction::Op(op, dst, a, b) =>
                dst.set(vars, match op {
                    Operator::Add => binary!(vars, a, b, +),
                    Operator::Sub => binary!(vars, a, b, -),
                    Operator::Mul => binary!(vars, a, b, *),
                    Operator::Div => binary!(vars, a, b, /),
                    Operator::Idiv => binary_i!(vars, a, b,
                        ?fn |a: i64, b: i64| Ok(
                            a
                            .checked_div(b)
                            .ok_or(VmError::DivisionByZero)? as f64
                        )),
                    Operator::Mod => binary!(vars, a, b, %),
                    Operator::Pow => binary!(vars, a, b, fn |a, b| f64::powf(a, b)),
                    Operator::Not => unary!(vars, a,
                        fn |a: f64| if a.abs() < f64::EPSILON { 1. } else { 0. }),
                    Operator::Land => binary!(vars, a, b,
                        !fn |a: f64, b: f64| a.abs() > f64::EPSILON && b.abs() > f64::EPSILON),
                    Operator::LessThan => binary!(vars, a, b, !fn |a: f64, b: f64| a < b),
                    Operator::LessThanEq => binary!(vars, a, b, !fn |a: f64, b: f64| a <= b),
                    Operator::GreaterThan => binary!(vars, a, b, !fn |a: f64, b: f64| a > b),
                    Operator::GreaterThanEq => binary!(vars, a, b, !fn |a: f64, b: f64| a >= b),
                    Operator::StrictEqual | Operator::Equal =>
                        Value::Num(if a.eval(vars)? == b.eval(vars)? { 1. } else { 0. }),
                    Operator::NotEqual =>
                        Value::Num(if a.eval(vars)? != b.eval(vars)? { 1. } else { 0. }),
                    Operator::Shl => binary_i!(vars, a, b, <<),
                    Operator::Shr => binary_i!(vars, a, b, >>),
                    Operator::Or => binary_i!(vars, a, b, |),
                    Operator::And => binary_i!(vars, a, b, &),
                    Operator::Xor => binary_i!(vars, a, b, ^),
                    Operator::Flip => unary!(vars, a, fn |a: f64| !(a as i64) as f64),
                    Operator::Max => binary!(vars, a, b, fn f64::max),
                    Operator::Min => binary!(vars, a, b, fn f64::min),
                    Operator::Abs => unary!(vars, a, fn f64::abs),
                    Operator::Log => unary!(vars, a, fn f64::ln),
                    Operator::Log10 => unary!(vars, a, fn f64::log10),
                    Operator::Floor => unary!(vars, a, fn f64::floor),
                    Operator::Ceil => unary!(vars, a, fn f64::ceil),
                    Operator::Sqrt => unary!(vars, a, fn f64::sqrt),
                    Operator::Angle => binary!(vars, a, b, fn |a: f64, b: f64| f64::atan2(b, a).to_degrees()),
                    Operator::Length => binary!(vars, a, b, fn |a: f64, b: f64| (a * a + b * b).sqrt()),
                    Operator::Sin => unary!(vars, a, fn |a: f64| a.to_radians().sin()),
                    Operator::Cos => unary!(vars, a, fn |a: f64| a.to_radians().cos()),
                    Operator::Tan => unary!(vars, a, fn |a: f64| a.to_radians().tan()),
                    Operator::Asin => unary!(vars, a, fn |a: f64| a.asin().to_degrees()),
                    Operator::Acos => unary!(vars, a, fn |a: f64| a.acos().to_degrees()),
                    Operator::Atan => unary!(vars, a, fn |a: f64| a.atan().to_degrees()),
                    Operator::Rand => unary!(vars, a,
                        fn |a: f64| random::<u32>() as f64 / u32::MAX as f64 * a),
                })?,

            Instruction::Wait(time) => {
                // only checks if parameter is a number
                time.eval(vars)?.as_num()?;
            },
            Instruction::Stop => return Ok(InstructionExecuteResult {
                halt: true
            }),
            Instruction::End => pc.set(vars, Value::Num(0.))?,
            Instruction::Jump(dst, op, a, b) =>
                if op == "always" || {
                    let a = a.eval(vars)?;
                    let b = b.eval(vars)?;
                    match op.as_str() {
                        "equal" | "strictEqual" => a == b,
                        "notEqual" => a != b,
                        op => {
                            let a = a.as_num()?;
                            let b = b.as_num()?;
                            match op {
                                "lessThan" => a < b,
                                "lessThanEq" => a <= b,
                                "greaterThan" => a > b,
                                "greaterThanEq" => a >= b,
                                _ => return Err(VmError::InvalidOperation(op.to_string())),
                            }
                        },
                    }
                } {
                    pc.set(vars, dst.eval(vars)?)?
                }
        }
        Ok(InstructionExecuteResult {
            halt: false,
        })
    }
}

#[test]
fn test_instruction_split_line() {
    assert_eq!(Instruction::split_line("a b c"), ["a", "b", "c"]);
    assert_eq!(Instruction::split_line("a \"b c d\" ef g"), ["a", "\"b c d\"", "ef", "g"]);
    assert_eq!(Instruction::split_line("va"), ["va"]);
    assert!(Instruction::split_line("").is_empty());
}
