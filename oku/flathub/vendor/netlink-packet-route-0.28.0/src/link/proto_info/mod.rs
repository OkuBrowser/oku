// SPDX-License-Identifier: MIT

mod bridge;
mod inet6;

pub use self::{bridge::LinkProtoInfoBridge, inet6::LinkProtoInfoInet6};
pub(crate) use self::{
    bridge::VecLinkProtoInfoBridge, inet6::VecLinkProtoInfoInet6,
};
