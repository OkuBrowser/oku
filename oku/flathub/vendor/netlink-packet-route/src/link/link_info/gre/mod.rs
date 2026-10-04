// SPDX-License-Identifier: MIT

mod gre_common;
pub mod info_gre;
pub mod info_gre6;

pub use self::{
    gre_common::{ErSpanDir, GreEncapFlags, GreEncapType, GreIOFlags},
    info_gre::InfoGre,
    info_gre6::InfoGre6,
};
