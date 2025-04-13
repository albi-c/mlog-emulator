use std::rc::Rc;
use std::str::FromStr;
use crate::building::Building;
use crate::value::Value;
use crate::variable::{VarHandle, Variables};
use crate::vm::{PrintBuffer, VmResult};

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

#[derive(Debug)]
pub enum Instruction {
    Read(VarHandle, ValueArg, ValueArg),
    Write(ValueArg, ValueArg, ValueArg),
    Print(ValueArg),
    PrintChar(ValueArg),
    Format(ValueArg),

    Set(VarHandle, ValueArg),

    PrintFlush(ValueArg),
    GetLink(VarHandle, ValueArg),
}

macro_rules! arg {
    (out, $vars:expr, $arg:expr) => ($vars.handle($arg));
    (in, $vars:expr, $arg:expr) => (ValueArg::parse($arg, $vars));
}

macro_rules! ins {
    ($ins:ident, $vars:expr, $args:expr => $($sel:tt $i:expr),*) => {
        Instruction::$ins($(arg!($sel, $vars, $args[$i])),*)
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
            "read" => ins!(Read, vars, args => out 1, in 2, in 2),
            "write" => ins!(Write, vars, args => in 1, in 2, in 3),
            "print" => ins!(Print, vars, args => in 1),
            "printchar" => ins!(PrintChar, vars, args => in 1),
            "format" => ins!(Format, vars, args => in 1),

            "set" => ins!(Set, vars, args => out 1, in 2),

            "printflush" => ins!(PrintFlush, vars, args => in 1),
            "getlink" => ins!(GetLink, vars, args => out 1, in 2),

            name => panic!("Invalid instruction: '{}'", name),
        })
    }

    pub fn execute(&self, vars: &Variables, print_buffer: &PrintBuffer,
                   buildings: &[Rc<dyn Building>]) -> VmResult<InstructionExecuteResult> {
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

            Instruction::Set(dst, src) =>
                dst.set(vars, src.eval(vars)?)?,

            Instruction::PrintFlush(val) =>
                val.eval(vars)?.as_building()?.print_flush(print_buffer.take())?,
            Instruction::GetLink(dst, idx) =>
                dst.set(vars, Value::Building(
                    idx.eval(vars)?.do_index(buildings, "get link")?.clone()))?,
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
