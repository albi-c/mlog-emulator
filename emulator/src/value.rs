use std::cell::OnceCell;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use crate::building::Building;
use crate::vm::{VmError, VmResult};

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Property(&'static str);

impl Property {
    pub const PROPERTIES: &'static [&'static str] = &["memoryCapacity", "size"];

    pub fn new(name: &'static str) -> Self {
        Property(name)
    }

    pub fn name(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Num(f64),
    Str(Rc<LazyUtf16String>),
    Building(Rc<dyn Building>),
    Property(Property),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Num(_) => "num",
            Value::Str(_) => "str",
            Value::Building(_) => "Building",
            Value::Property(_) => "Property",
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

    pub fn as_int(&self) -> VmResult<i64> {
        let num = self.as_num()?;
        if (num.round() - num).abs() < f64::EPSILON {
            Ok(num as i64)
        } else {
            Err(self._invalid_cast("int"))
        }
    }

    pub fn as_index(&self, len: usize, device: &'static str) -> VmResult<usize> {
        let val = self.as_int()?;
        if val < 0 {
            return Err(VmError::NegativeIndex(val, device));
        }
        let val = val as usize;
        if val >= len {
            return Err(VmError::IndexTooHigh(val, len, device));
        }
        Ok(val)
    }

    pub fn do_index<'a, T>(&self, data: &'a [T], device: &'static str) -> VmResult<&'a T> {
        Ok(&data[self.as_index(data.len(), device)?])
    }

    pub fn do_index_copy<T: Copy>(&self, data: &[T], device: &'static str) -> VmResult<T> {
        Ok(data[self.as_index(data.len(), device)?])
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

    pub fn as_property(&self) -> VmResult<Property> {
        match self {
            Value::Property(property) => Ok(*property),
            _ => Err(self._invalid_cast("Property")),
        }
    }

    pub fn sense(&self, property: Property) -> VmResult<Value> {
        match self {
            Value::Str(string) => if property.name() == "size" {
                return Ok(Value::Num(string.as_utf_16().len() as f64));
            },
            Value::Building(building) => return building.sense(property),
            _ => {},
        }
        Ok(Value::Null)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Num(num) => write!(f, "{}", num),
            Value::Str(string) => write!(f, "{}", string),
            Value::Building(building) => write!(f, "{}", building.name()),
            Value::Property(property) => write!(f, "@{}", property.name()),
        }
    }
}
