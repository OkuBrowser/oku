// SPDX-License-Identifier: MIT

pub(crate) fn expand_buffer_if_small(
    got: &[u8],
    expected_size: usize,
    nla_name: &str,
) -> Vec<u8> {
    match got.len().cmp(&expected_size) {
        std::cmp::Ordering::Greater => {
            log::debug!(
                "Specified {nla_name} NLA attribute holds more(most likely \
                 new kernel) data which is unknown to netlink-packet-route \
                 crate, expecting {expected_size}, got {}",
                got.len()
            );
        }
        std::cmp::Ordering::Less => {
            let mut payload = got.to_vec();
            payload.resize(expected_size, 0);
            return payload;
        }
        std::cmp::Ordering::Equal => {}
    }
    got.to_vec()
}
