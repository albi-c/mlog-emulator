use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::rc::Rc;
use crate::building::Building;
use crate::value::{LazyUtf16String, Value};
use crate::vm::{VmError, VmResult};

#[derive(Debug)]
pub struct Variable {
    name: Rc<String>,
    value: Value,
    constant: bool,
}

impl Variable {
    pub fn new_const(name: Rc<String>, value: Value, constant: bool) -> Self {
        Variable {
            name,
            value,
            constant,
        }
    }

    pub fn new(name: Rc<String>, value: Value) -> Self {
        Self::new_const(name, value, false)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn val(&self) -> &Value {
        &self.value
    }

    pub fn val_mut(&mut self) -> VmResult<&mut Value> {
        if self.constant {
            Err(VmError::ConstantMutation(self.name.to_string()))
        } else {
            Ok(&mut self.value)
        }
    }

    pub fn constant(&self) -> bool {
        self.constant
    }

    pub fn clone_as(&self, name: Rc<String>) -> Self {
        Self::new_const(name, self.value.clone(), self.constant)
    }

    pub fn into_value(self) -> Value {
        self.value
    }

    pub fn type_name(&self) -> &'static str {
        self.value.type_name()
    }

    pub fn is_null(&self) -> bool {
        self.value.is_null()
    }

    fn _invalid_cast(&self, to: &'static str) -> VmError {
        VmError::InvalidCast(self.name.to_string(), self.type_name(), to)
    }

    pub fn as_num(&self) -> VmResult<f64> {
        self.value.as_num().map_err(
            |_| self._invalid_cast("num"))
    }

    pub fn as_str(&self) -> VmResult<&Rc<LazyUtf16String>> {
        self.value.as_str().map_err(
            |_| self._invalid_cast("str"))
    }

    pub fn as_building(&self) -> VmResult<&Rc<dyn Building>> {
        self.value.as_building().map_err(
            |_| self._invalid_cast("Building"))
    }
}

#[derive(Debug)]
pub struct Variables {
    variables: HashMap<Rc<String>, Variable>,
}

impl Variables {
    pub fn get(&self, name: &String) -> VmResult<&Variable> {
        self.variables.get(name).ok_or_else(|| VmError::VariableNotFound(name.to_string()))
    }
    pub fn set(&mut self, name: Rc<String>, value: Value) -> VmResult<()> {
        match self.variables.entry(name.clone()) {
            Occupied(entry) => {
                *entry.into_mut().val_mut()? = value;
                Ok(())
            },
            Vacant(entry) => {
                entry.insert(Variable::new(name, value));
                Ok(())
            },
        }
    }
    pub fn insert(&mut self, name: &str, value: Value) -> VmResult<()> {
        self.set(Rc::new(name.to_string()), value)
    }
}

impl<const N: usize> From<[(Rc<String>, Variable); N]> for Variables {
    fn from(value: [(Rc<String>, Variable); N]) -> Self {
        Variables {
            variables: HashMap::from(value),
        }
    }
}
