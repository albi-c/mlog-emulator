use std::rc::Rc;
use std::str::FromStr;
use crate::value::Value;
use crate::variable::Variables;
use crate::vm::VmResult;

#[derive(Debug)]
pub enum ValueArg {
    Value(Value),
    Variable(String),
}

impl ValueArg {
    fn parse(string: &str) -> Self {
        if string.starts_with("\"") && string.ends_with("\"") {
            ValueArg::Value(Value::Str(Rc::new(string[1..string.len()-1].into())))
        } else if let Ok(num) = f64::from_str(string) {
            ValueArg::Value(Value::Num(num))
        } else {
            ValueArg::Variable(string.to_string())
        }
    }

    pub fn eval(&self, vars: &Variables) -> VmResult<Value> {
        match self {
            ValueArg::Value(val) => Ok(val.clone()),
            ValueArg::Variable(name) =>
                vars.get(name).map(|var| var.val().clone()),
        }
    }
}

impl From<&str> for ValueArg {
    fn from(value: &str) -> Self {
        ValueArg::parse(value)
    }
}

#[derive(Debug)]
pub enum Instruction {
    Set(Rc<String>, ValueArg),
    Print(ValueArg),
    PrintFlush(ValueArg),
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

    pub fn parse(line: &str) -> Option<Self> {
        let spl = Self::split_line(line);
        if spl.is_empty() {
            return None;
        }
        Some(match spl[0] {
            "set" => Instruction::Set(Rc::new(spl[1].to_string()), spl[2].into()),
            "print" => Instruction::Print(spl[1].into()),
            "printflush" => Instruction::PrintFlush(spl[1].into()),
            name => panic!("Invalid instruction: '{}'", name),
        })
    }

    pub fn execute(&self, vars: &mut Variables, print_buffer: &mut String) -> VmResult<()> {
        match self {
            Instruction::Set(dst, src) =>
                vars.set(dst.clone(), src.eval(vars)?)?,
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
