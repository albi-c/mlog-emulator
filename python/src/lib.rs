use std::collections::HashMap;
use pyo3::prelude::*;
use emulator::interface;
use emulator::interface::Output;
use emulator::vm::VmFinishReason;

#[pyclass]
#[derive(Debug, Clone)]
enum Device {
    Message(),
    Memory(usize),
}

#[pyclass]
#[derive(Debug, Copy, Clone)]
enum FinishReason {
    PcWrap,
    Halt,
    InsLimit,
}

#[pyclass]
#[derive(Debug, Clone)]
enum DeviceState {
    Message {
        text: String,
    },
    Memory {
        data: Vec<f64>,
    },
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum ErrorPos {
    Instruction(usize),
    None(),
    PcFetch(),
}

#[pyclass]
#[derive(Debug, Clone)]
enum ExecutionResult {
    Success {
        finish_reason: FinishReason,
        devices: HashMap<String, DeviceState>,
        print_buffer: String,
    },
    Failure {
        pos: ErrorPos,
        msg: String,
    },
}

#[pyclass]
#[derive(Debug, Clone)]
struct Executor {
    #[pyo3(set)]
    code: String,
    #[pyo3(set)]
    code_len_limit: Option<usize>,
    #[pyo3(set)]
    instruction_limit: Option<usize>,
    #[pyo3(set)]
    end_on_wrap: bool,
    devices: Vec<(String, interface::Device)>,
}

impl Executor {
    fn get_options(&mut self) -> interface::Options {
        interface::Options {
            code: std::mem::take(&mut self.code),
            code_len_limit: self.code_len_limit,
            instruction_limit: self.instruction_limit,
            end_on_wrap: self.end_on_wrap,
            devices: std::mem::take(&mut self.devices),
        }
    }
}

#[pymethods]
impl Executor {
    #[new]
    pub fn new(code: String) -> Self {
        Executor {
            code,
            code_len_limit: None,
            instruction_limit: None,
            end_on_wrap: true,
            devices: vec![],
        }
    }

    pub fn add_device(&mut self, name: String, device: Device) {
        self.devices.push((name, match device {
            Device::Message() => interface::Device::Message,
            Device::Memory(capacity) => interface::Device::Memory(capacity),
        }));
    }

    pub fn execute(&mut self) -> ExecutionResult {
        match interface::run_from_options(self.get_options()) {
            Output::Success { finish_reason, devices, print_buffer } => ExecutionResult::Success {
                finish_reason: match finish_reason {
                    VmFinishReason::PcWrap => FinishReason::PcWrap,
                    VmFinishReason::Halt => FinishReason::Halt,
                    VmFinishReason::InsLimit => FinishReason::InsLimit,
                },
                devices: devices.into_iter().map(|(k, v)| (k, match v {
                    interface::DeviceState::Message(text) => DeviceState::Message { text },
                    interface::DeviceState::Memory(data) =>
                        DeviceState::Memory { data: data.to_vec() },
                })).collect(),
                print_buffer,
            },
            Output::Failure { pos, msg } => ExecutionResult::Failure {
                pos: match pos {
                    interface::ErrorPos::Instruction(i) => ErrorPos::Instruction(i),
                    interface::ErrorPos::None => ErrorPos::None(),
                    interface::ErrorPos::PcFetch => ErrorPos::PcFetch(),
                },
                msg,
            },
        }
    }

    pub fn execute_to_json(&mut self) -> String {
        let result = interface::run_from_options(self.get_options());
        serde_json::to_string(&result).unwrap()
    }
}

#[pymodule]
fn mlog_emulator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Device>()?;
    m.add_class::<Executor>()?;
    m.add_class::<FinishReason>()?;
    m.add_class::<DeviceState>()?;
    m.add_class::<ErrorPos>()?;
    m.add_class::<ExecutionResult>()?;
    Ok(())
}
