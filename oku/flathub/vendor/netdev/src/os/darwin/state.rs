use crate::interface::state::OperState;

pub fn operstate(if_name: &str) -> OperState {
    match super::flags::get_interface_flags(if_name) {
        Ok(flags) => OperState::from_if_flags(flags),
        Err(_) => OperState::Unknown,
    }
}
