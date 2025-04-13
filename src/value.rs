use std::cell::OnceCell;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use crate::building::Building;
use crate::vm::{VmError, VmResult};

#[derive(Debug, Clone)]
pub struct LazyUtf16String {
    string: Rc<String>,
    utf_16: OnceCell<Vec<u16>>,
}

impl LazyUtf16String {
    pub fn new(string: Rc<String>) -> Self {
        LazyUtf16String {
            string,
            utf_16: OnceCell::new(),
        }
    }

    pub fn as_utf_16(&self) -> &[u16] {
        self.utf_16.get_or_init(|| self.string.encode_utf16().collect())
    }

    pub fn to_utf_16(mut self) -> Vec<u16> {
        self.utf_16.take().unwrap_or_else(|| self.string.encode_utf16().collect())
    }

    pub fn as_string_ref(&self) -> &String {
        &self.string
    }

    pub fn clone_string(&self) -> Rc<String> {
        self.string.clone()
    }
}

impl From<String> for LazyUtf16String {
    fn from(value: String) -> Self {
        LazyUtf16String::new(Rc::new(value))
    }
}

impl From<Rc<String>> for LazyUtf16String {
    fn from(value: Rc<String>) -> Self {
        LazyUtf16String::new(value)
    }
}

impl From<&str> for LazyUtf16String {
    fn from(value: &str) -> Self {
        LazyUtf16String::new(Rc::new(value.to_string()))
    }
}

impl From<LazyUtf16String> for String {
    fn from(value: LazyUtf16String) -> Self {
        value.string.to_string()
    }
}

impl From<LazyUtf16String> for Rc<String> {
    fn from(value: LazyUtf16String) -> Self {
        value.string
    }
}

impl Deref for LazyUtf16String {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl Display for LazyUtf16String {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Num(f64),
    Str(Rc<LazyUtf16String>),
    Building(Rc<dyn Building>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Num(_) => "num",
            Value::Str(_) => "str",
            Value::Building(_) => "Building",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn _invalid_cast(&self, to: &'static str) -> VmError {
        VmError::InvalidCast(self.to_string(), self.type_name(), to)
    }

    pub fn as_num(&self) -> VmResult<f64> {
        match self {
            Value::Num(num) => Ok(*num),
            _ => Err(self._invalid_cast("num")),
        }
    }

    pub fn as_str(&self) -> VmResult<Rc<LazyUtf16String>> {
        match self {
            Value::Str(string) => Ok(string.clone()),
            _ => Err(self._invalid_cast("str")),
        }
    }

    pub fn as_building(&self) -> VmResult<Rc<dyn Building>> {
        match self {
            Value::Building(building) => Ok(building.clone()),
            _ => Err(self._invalid_cast("Building")),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Num(num) => write!(f, "{}", num),
            Value::Str(string) => write!(f, "{}", string),
            Value::Building(building) => write!(f, "{}", building.name()),
        }
    }
}
