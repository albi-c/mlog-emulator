use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Weak;
use crate::value::Value;
use crate::variable::Variables;
use crate::vm::{VmError, VmResult};

pub trait Building : Debug {
    fn name(&self) -> &str;

    fn print_flush(&self, _string: String) -> VmResult<()> {
        Err(VmError::InvalidBuildingType("print flush into", self.name().to_string()))
    }

    fn read(&self, _index: Value) -> VmResult<Value> {
        Err(VmError::InvalidBuildingType("read from", self.name().to_string()))
    }
    fn write(&self, _index: Value, _value: Value) -> VmResult<()> {
        Err(VmError::InvalidBuildingType("write into", self.name().to_string()))
    }
}

#[derive(Debug)]
pub struct ProcessorBuilding {
    name: String,
    variables: Weak<Variables>,
}

impl ProcessorBuilding {
    pub fn new(name: String, variables: Weak<Variables>) -> Self {
        ProcessorBuilding {
            name,
            variables,
        }
    }
}

impl Building for ProcessorBuilding {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&self, index: Value) -> VmResult<Value> {
        let index = index.as_str()?;
        let vars = self.variables.upgrade().unwrap();
        vars.get_handle(index.as_string_ref())
            .ok_or_else(|| VmError::VariableNotFound(index.to_string()))
            .map(|h| h.val(&vars).clone())
    }
    fn write(&self, index: Value, value: Value) -> VmResult<()> {
        let index = index.as_str()?;
        let vars = self.variables.upgrade().unwrap();
        vars.get_handle(index.as_string_ref())
            .ok_or_else(|| VmError::VariableNotFound(index.to_string()))
            .map(|h| h.set(&vars, value))?
    }
}

#[derive(Debug)]
pub struct MessageBuilding {
    name: String,
    text: RefCell<String>,
}

impl MessageBuilding {
    pub fn new(name: String) -> Self {
        MessageBuilding {
            name,
            text: RefCell::new("".to_string()),
        }
    }

    pub fn get_text(&self) -> String {
        self.text.clone().into_inner()
    }
}

impl Building for MessageBuilding {
    fn name(&self) -> &str {
        &self.name
    }

    fn print_flush(&self, string: String) -> VmResult<()> {
        *self.text.borrow_mut() = string;
        Ok(())
    }
}
