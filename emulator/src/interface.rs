use std::collections::HashMap;
use std::io::{Read, Write};
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use crate::building::{Building, MemoryBuilding, MessageBuilding};
use crate::vm::{VmError, VmFinishReason, VM};

#[derive(Debug, Clone, Deserialize)]
pub enum Device {
    Message,
    Memory(usize),
}

impl Device {
    pub fn construct(self, name: String) -> (Rc<dyn Building>, Box<dyn FnOnce() -> DeviceState>) {
        match self {
            Device::Message => {
                let dev = Rc::new(MessageBuilding::new(name));
                (dev.clone(), Box::new(move || DeviceState::Message(dev.get_text())))
            },
            Device::Memory(capacity) => {
                let dev = Rc::new(MemoryBuilding::new(name, capacity));
                (dev.clone(), Box::new(move || DeviceState::Memory(dev.get_data())))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Options {
    pub code: String,
    pub code_len_limit: Option<usize>,
    pub instruction_limit: Option<usize>,
    pub end_on_wrap: bool,
    pub devices: Vec<(String, Device)>,
}

#[derive(Debug, Serialize)]
pub enum DeviceState {
    Message(String),
    Memory(Box<[f64]>),
}

#[derive(Debug, Serialize)]
pub enum ErrorPos {
    Instruction(usize),
    None,
    PcFetch
}

#[derive(Debug, Serialize)]
pub enum Output {
    Success {
        finish_reason: VmFinishReason,
        devices: HashMap<String, DeviceState>,
        print_buffer: String,
    },
    Failure {
        pos: ErrorPos,
        msg: String,
    },
}

pub fn run_from_options(options: Options) -> Output {
    let mut devices = vec![];
    let mut device_state_getters = vec![];
    for (name, device) in options.devices {
        let (device, getter) = device.construct(name.clone());
        devices.push(device);
        device_state_getters.push((name, getter));
    }

    let vm = VM::new(
        &options.code,
        options.code_len_limit.unwrap_or(VM::DEFAULT_CODE_LEN_LIMIT),
        devices,
    ).unwrap();
    match vm.run(options.instruction_limit, options.end_on_wrap) {
        Ok(finish_reason) => Output::Success {
            finish_reason,
            devices: device_state_getters
                .into_iter()
                .map(|(name, getter)| (name, getter()))
                .collect(),
            print_buffer: vm.into_print_buffer().take(),
        },
        Err(err) => Output::Failure {
            pos: match &err.1 {
                Some(pos) => ErrorPos::Instruction(*pos),
                None => match &err.0 {
                    VmError::PcResError(_) => ErrorPos::PcFetch,
                    _ => ErrorPos::None,
                },
            },
            msg: err.to_string(),
        },
    }
}

pub fn run_from_json(input: impl Read, output: impl Write) {
    let options: Options = serde_json::from_reader(input).unwrap();
    let result = run_from_options(options);
    serde_json::to_writer(output, &result).unwrap();
}
