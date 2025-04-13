#![feature(unsafe_cell_access)]

use std::rc::Rc;
use crate::building::MessageBuilding;
use crate::vm::VM;

mod vm;
mod value;
mod building;
mod variable;
mod instruction;

const CODE: &'static str = r#"
set @counter 0.5
set x 3
read x "abd" x
print x
print "abc dd ef"
printflush message1
"#;

fn main() {
    let message1= Rc::new(MessageBuilding::new("message1".to_string()));
    let vm = VM::new(CODE, VM::DEFAULT_CODE_LEN_LIMIT, vec![message1.clone()]).unwrap();
    match vm.run(Some(1000), true) {
        Ok(_) => {
            println!("{}", message1.get_text());
        },
        Err(err) => {
            println!("{}", err);
        },
    }
}
