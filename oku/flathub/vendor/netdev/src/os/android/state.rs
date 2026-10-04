use crate::interface::state::OperState;

pub fn operstate(if_name: &str) -> OperState {
    match super::netlink::get_flags_by_name(if_name) {
        Ok(Some(flags)) => OperState::from_if_flags(flags),
        Ok(None) => OperState::Unknown,
        Err(_) => OperState::Unknown,
    }
}
