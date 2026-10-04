// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable as _};

use crate::{
    tc::{
        TcAttribute, TcBpfFlags, TcFilterBpfOption, TcHandle, TcHeader,
        TcMessage, TcMessageBuffer, TcOption, TcU32OptionFlags,
    },
    AddressFamily,
};

// TC eBPF program attaches to the clsact qdisc.
// The `bpf_prog.o` is the compiled ELF file.
//
// Setup:
//      ip link add dummy0 type dummy
//      ip link set dummy0 up
//      tc qdisc add dev dummy0 clsact
//      tc filter add dev dummy0 egress bpf obj bpf_prog.o sec tc
//
// Capture nlmon of this command:
//
//      tc -s filter show dev dummy0 egress
//
// Raw packet modification:
//   * rtnetlink header removed.
#[test]
fn test_get_filter_bpf() {
    let raw = vec![
        0x00, // tcm_family: AF_UNSPEC
        0x00, 0x00, 0x00, // tcm__pad1 & tcm__pad2
        0x05, 0x00, 0x00, 0x00, // tcm_ifindex: 5
        0x00, 0x00, 0x00, 0x00, // tcm_handle: 0:0
        0xf3, 0xff, 0xff, 0xff, // tcm_parent: TC_H_CLSACT:TC_H_MIN_EGRESS
        0x00, 0x00, 0x00, 0x00, // tcm_info: 0
        0x08, 0x00, // NLA length: 8
        0x01, 0x00, // NLA type: TCA_KIND
        0x62, 0x70, 0x66, 0x00, // NLA data: "bpf\0"
        0x08, 0x00, // NLA length: 8
        0x0b, 0x00, // NLA type: TCA_CHAIN
        0x00, 0x00, 0x00, 0x00, // NLA data: 0
        0x48, 0x00, // NLA length: 72
        0x02, 0x00, // NLA type: TCA_OPTIONS
        0x08, 0x00, // NLA length: 8
        0x03, 0x00, // NLA type: TCA_BPF_CLASSID
        0x06, 0x00, 0x07, 0x00, // NLA data: 7:6
        0x08, 0x00, // NLA length: 8
        0x06, 0x00, // NLA type: TCA_BPF_FD
        0x04, 0x00, 0x00, 0x00, // NLA data: 4
        0x0e, 0x00, // NLA length: 14
        0x07, 0x00, // NLA type: TCA_BPF_NAME
        0x70, 0x61, 0x72, 0x73, 0x65, 0x5f, 0x73, 0x6b, 0x62, 0x00, 0x00,
        0x00, // NLA data: "parse_skb\0" and 2 bytes pad
        0x08, 0x00, // NLA length: 8
        0x08, 0x00, // NLA type: TCA_BPF_FLAGS
        0x01, 0x00, 0x00, 0x00, // NLA data: TCA_BPF_FLAG_ACT_DIRECT
        0x08, 0x00, // NLA length: 8
        0x09, 0x00, // NLA type: TCA_BPF_FLAGS_GEN
        0x08, 0x00, 0x00, 0x00, // NLA data: TCA_CLS_FLAGS_NOT_IN_HW
        0x0c, 0x00, // NLA length: 12
        0x0a, 0x00, // NLA type: TCA_BPF_TAG
        0xa0, 0x4f, 0x5e, 0xef, 0x06, 0xa7, 0xf5,
        0x55, // NLA data: a04f5eef06a7f555
        0x08, 0x00, // NLA length: 8
        0x0b, 0x00, // NLA type: TCA_BPF_ID
        0x27, 0x00, 0x00, 0x00, // NLA data: 39
    ];

    // TC_H_MAKE(TC_H_CLSACT, TC_H_MIN_EGRESS)
    let mut egress = TcHandle::CLSACT;
    egress.minor = TcHandle::MIN_EGRESS;

    let expected = TcMessage {
        header: TcHeader {
            family: AddressFamily::Unspec,
            index: 5,
            handle: TcHandle::UNSPEC,
            parent: egress,
            info: 0,
        },
        attributes: vec![
            TcAttribute::Kind("bpf".to_string()),
            TcAttribute::Chain(0),
            TcAttribute::Options(vec![
                TcOption::Bpf(TcFilterBpfOption::ClassId(TcHandle {
                    major: 7,
                    minor: 6,
                })),
                TcOption::Bpf(TcFilterBpfOption::ProgFd(4)),
                TcOption::Bpf(TcFilterBpfOption::ProgName(
                    "parse_skb".to_string(),
                )),
                TcOption::Bpf(TcFilterBpfOption::Flags(
                    TcBpfFlags::DirectAction,
                )),
                TcOption::Bpf(TcFilterBpfOption::FlagsGeneric(
                    TcU32OptionFlags::NotInHw,
                )),
                TcOption::Bpf(TcFilterBpfOption::ProgTag([
                    0xa0, 0x4f, 0x5e, 0xef, 0x06, 0xa7, 0xf5, 0x55,
                ])),
                TcOption::Bpf(TcFilterBpfOption::ProgId(39)),
            ]),
        ],
    };

    assert_eq!(
        expected,
        TcMessage::parse(&TcMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
