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
set __tmp3:2 0
jump 7 greaterThanEq __tmp3:2 @links
getlink __tmp5:2 __tmp3:2
print __tmp5:2
print "\n"
op add __tmp3:2 __tmp3:2 1
jump 1 always _ _
printflush message1
"#;

fn main() {
    let message1= Rc::new(MessageBuilding::new("message1".to_string()));
    let message2= Rc::new(MessageBuilding::new("message2".to_string()));
    let vm = VM::new(CODE, VM::DEFAULT_CODE_LEN_LIMIT,
                     vec![message1.clone(), message2.clone()]).unwrap();
    match vm.run(Some(1000), true) {
        Ok(_) => {
            println!("{}", message1.get_text());
        },
        Err(err) => {
            println!("{}", err);
        },
    }
}
