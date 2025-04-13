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
    variables: Weak<RefCell<Variables>>,
}

impl ProcessorBuilding {
    pub fn new(name: String, variables: Weak<RefCell<Variables>>) -> Self {
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
        self.variables.upgrade().unwrap().borrow()
            .get(index.as_string_ref()).map(|var| var.val().clone())
    }
    fn write(&self, index: Value, value: Value) -> VmResult<()> {
        let index = index.as_str()?;
        self.variables.upgrade().unwrap().borrow_mut()
            .set(index.clone_string(), value)
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
