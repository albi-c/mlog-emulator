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

    pub fn as_num(&self) -> VmResult<f64> {
        self.value.as_num()
    }

    pub fn as_str(&self) -> VmResult<&Rc<LazyUtf16String>> {
        self.value.as_str()
    }

    pub fn as_building(&self) -> VmResult<&Rc<dyn Building>> {
        self.value.as_building()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VarHandle(usize);

impl VarHandle {
    pub fn get(self, vars: &Variables) -> &Variable {
        &vars.variables[self.0]
    }
    pub fn val(self, vars: &Variables) -> &Value {
        self.get(vars).val()
    }
    pub fn set(self, vars: &mut Variables, value: Value) -> VmResult<()> {
        *vars.variables[self.0].val_mut()? = value;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Variables {
    variables: Vec<Variable>,
    by_name: HashMap<String, usize>,
}

impl Variables {
    pub fn handle(&mut self, name: &str) -> VarHandle {
        if let Some(idx) = self.by_name.get(name) {
            VarHandle(*idx)
        } else {
            let name = Rc::new(name.clone());
            let idx = self.variables.len();
            self.by_name.insert(name.to_string(), idx);
            self.variables.push(Variable::new(Rc::new(name.to_string()), Value::Null));
            VarHandle(idx)
        }
    }
    
    pub fn get_handle(&self, name: &str) -> Option<VarHandle> {
        self.by_name.get(name).map(|idx| VarHandle(*idx))
    }

    pub fn insert(&mut self, name: String, var: Variable) -> VarHandle {
        match self.by_name.entry(name) {
            Occupied(entry) => panic!("Double insert: {}", entry.key()),
            Vacant(entry) => {
                let idx = self.variables.len();
                entry.insert(idx);
                self.variables.push(var);
                VarHandle(idx)
            },
        }
    }
}

impl<const N: usize> From<[(&str, Variable); N]> for Variables {
    fn from(value: [(&str, Variable); N]) -> Self {
        let mut vars = Variables {
            variables: vec![],
            by_name: HashMap::new(),
        };
        for (name, var) in value {
            vars.insert(name.to_string(), var);
        }
        vars
    }
}
