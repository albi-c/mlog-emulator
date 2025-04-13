use std::rc::Rc;
use std::str::FromStr;
use crate::value::Value;
use crate::variable::{VarHandle, Variables};
use crate::vm::VmResult;

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

    pub fn eval<'a>(&'a self, vars: &'a Variables) -> VmResult<&'a Value> {
        Ok(match self {
            ValueArg::Value(val) => val,
            ValueArg::Variable(var) => var.val(vars),
        })
    }
}

#[derive(Debug)]
pub enum Instruction {
    Set(VarHandle, ValueArg),
    Print(ValueArg),
    PrintFlush(ValueArg),
}

macro_rules! va {
    ($vars:expr, $arg:expr) => (ValueArg::parse($arg, $vars));
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
        let spl = Self::split_line(line);
        if spl.is_empty() {
            return None;
        }
        Some(match spl[0] {
            "set" => Instruction::Set(vars.handle(spl[1]), va!(vars, spl[2])),
            "print" => Instruction::Print(va!(vars, spl[1])),
            "printflush" => Instruction::PrintFlush(va!(vars, spl[1])),
            name => panic!("Invalid instruction: '{}'", name),
        })
    }

    pub fn execute(&self, vars: &mut Variables, print_buffer: &mut String) -> VmResult<()> {
        match self {
            Instruction::Set(dst, src) =>
                dst.set(vars, src.eval(vars)?.clone())?,
            Instruction::Print(val) =>
                print_buffer.push_str(&val.eval(vars)?.to_string()),
            Instruction::PrintFlush(val) =>
                val.eval(vars)?.as_building()?.print_flush(
                    std::mem::replace(print_buffer, "".to_string()))?,
        }
        Ok(())
    }
}

#[test]
fn test_instruction_split_line() {
    assert_eq!(Instruction::split_line("a b c"), ["a", "b", "c"]);
    assert_eq!(Instruction::split_line("a \"b c d\" ef g"), ["a", "\"b c d\"", "ef", "g"]);
    assert_eq!(Instruction::split_line("va"), ["va"]);
    assert!(Instruction::split_line("").is_empty());
}
