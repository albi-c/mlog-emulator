use crate::building::MessageBuilding;
use crate::vm::VM;

mod vm;
mod value;
mod building;
mod variable;
mod instruction;

const CODE: &'static str = r#"
set x 1
print x
print "abc dd ef"
printflush message1
"#;

fn main() {
    let mut vm = VM::new(CODE, VM::DEFAULT_CODE_LEN_LIMIT).unwrap();
    let message1 = vm.building(MessageBuilding::new("message1".to_string())).unwrap();
    for _ in 0..1000 {
        vm.cycle().unwrap();
    }
    println!("{}", message1.get_text());
}
