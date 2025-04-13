use std::collections::HashMap;
use std::io::{Read, Write};
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use crate::building::{Building, MessageBuilding};
use crate::vm::{VmError, VmFinishReason, VM};

#[derive(Debug, Deserialize)]
enum Device {
    Message,
}

impl Device {
    fn construct(self, name: String) -> (Rc<dyn Building>, Box<dyn FnOnce() -> DeviceState>) {
        match self {
            Device::Message => {
                let dev = Rc::new(MessageBuilding::new(name));
                (dev.clone(), Box::new(move || DeviceState::Message(dev.get_text())))
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct Options {
    code: String,
    code_len_limit: Option<usize>,
    instruction_limit: Option<usize>,
    end_on_wrap: bool,
    devices: HashMap<String, Device>,
}

#[derive(Debug, Serialize)]
enum DeviceState {
    Message(String),
}

#[derive(Debug, Serialize)]
enum ErrorPos {
    Instruction(usize),
    None,
    PcFetch
}

#[derive(Debug, Serialize)]
enum Output {
    Success {
        finish_reason: VmFinishReason,
        devices: HashMap<String, DeviceState>,
    },
    Failure {
        pos: ErrorPos,
        msg: String,
    },
}

pub fn run_from_json(input: impl Read, output: impl Write) {
    let options: Options = serde_json::from_reader(input).unwrap();

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
    let result = match vm.run(options.instruction_limit, options.end_on_wrap) {
        Ok(finish_reason) => Output::Success {
            finish_reason,
            devices: device_state_getters
                .into_iter()
                .map(|(name, getter)| (name, getter()))
                .collect(),
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
    };

    serde_json::to_writer(output, &result).unwrap();
}
