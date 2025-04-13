use std::cell::UnsafeCell;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::rc::Rc;
use crate::building::Building;
use crate::value::{LazyUtf16String, Value};
use crate::vm::{VmError, VmResult};

#[derive(Debug)]
pub struct Variable {
    name: Rc<String>,
    value: UnsafeCell<Value>,
    constant: bool,
}

impl Variable {
    pub fn new_const(name: Rc<String>, value: Value, constant: bool) -> Self {
        Variable {
            name,
            value: UnsafeCell::new(value),
            constant,
        }
    }

    pub fn new(name: Rc<String>, value: Value) -> Self {
        Self::new_const(name, value, false)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn val(&self) -> Value {
        unsafe { self.value.as_ref_unchecked() }.clone()
    }

    pub fn set_val(&self, value: Value) -> VmResult<()> {
        if self.constant {
            Err(VmError::ConstantMutation(self.name.to_string()))
        } else {
            unsafe { self.value.replace(value) };
            Ok(())
        }
    }

    pub fn force_set_val(&self, value: Value) {
        unsafe { self.value.replace(value) };
    }

    pub fn constant(&self) -> bool {
        self.constant
    }

    pub fn clone_as(&self, name: Rc<String>) -> Self {
        Self::new_const(name, self.val(), self.constant)
    }

    pub fn into_value(self) -> Value {
        self.value.into_inner()
    }

    pub fn type_name(&self) -> &'static str {
        unsafe { self.value.as_ref_unchecked() }.type_name()
    }

    pub fn is_null(&self) -> bool {
        unsafe { self.value.as_ref_unchecked() }.is_null()
    }

    pub fn as_num(&self) -> VmResult<f64> {
        unsafe { self.value.as_ref_unchecked() }.as_num()
    }

    pub fn as_str(&self) -> VmResult<Rc<LazyUtf16String>> {
        unsafe { self.value.as_ref_unchecked() }.as_str()
    }

    pub fn as_building(&self) -> VmResult<Rc<dyn Building>> {
        unsafe { self.value.as_ref_unchecked() }.as_building()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VarHandle(usize);

impl VarHandle {
    pub fn get(self, vars: &Variables) -> &Variable {
        &vars.variables[self.0]
    }
    pub fn val(self, vars: &Variables) -> Value {
        self.get(vars).val()
    }
    pub fn set(self, vars: &Variables, value: Value) -> VmResult<()> {
        vars.variables[self.0].set_val(value)
    }
    pub fn force_set(self, vars: &Variables, value: Value) {
        vars.variables[self.0].force_set_val(value);
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
