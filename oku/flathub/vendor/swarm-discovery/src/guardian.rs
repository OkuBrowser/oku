use crate::{
    receiver::{receiver, ReceiverError},
    sender::{self, sender},
    socket::Sockets,
    updater::updater,
    Discoverer,
};
use acto::{AcTokioRuntime, ActoCell, ActoInput, ActoRef};
use hickory_proto::rr::Name;
use std::{collections::HashMap, mem::replace, net::IpAddr};

pub enum Input {
    RemoveAll,
    RemovePort(u16),
    RemoveAddr(IpAddr),
    AddAddr(u16, Vec<IpAddr>),
    SetTxt(String, Option<String>),
    RemoveTxt(String),
    AddInterface(IpAddr),
    RemoveInterface(IpAddr),
}

pub async fn guardian(
    mut ctx: ActoCell<Input, AcTokioRuntime, Result<(), ReceiverError>>,
    mut discoverer: Discoverer,
    sockets: Sockets,
    service_name: Name,
) {
    let callback = replace(&mut discoverer.callback, Box::new(|_, _| {}));
    let tau = discoverer.tau;
    let phi = discoverer.phi;
    let upd_ref = ctx.supervise(
        ctx.spawn("updater", move |ctx| updater(ctx, tau, phi, callback))
            .map_handle(Ok),
    );

    let sockets2 = sockets.clone();
    let sn = service_name.clone();
    let snd_ref = ctx.supervise(
        ctx.spawn("sender", move |ctx| {
            sender(ctx, sockets, upd_ref, discoverer, sn)
        })
        .map_handle(Ok),
    );

    if let Some(v4) = sockets2.v4() {
        let service_name = service_name.clone();
        let snd_ref = snd_ref.clone();
        ctx.spawn_supervised("receiver_v4", move |ctx| {
            receiver(ctx, service_name, v4, snd_ref)
        });
    }

    if let Some(v6) = sockets2.v6() {
        let service_name = service_name.clone();
        let snd_ref = snd_ref.clone();
        ctx.spawn_supervised("receiver_v6", move |ctx| {
            receiver(ctx, service_name, v6, snd_ref)
        });
    }

    // Track interface receivers so we can stop them when interfaces are removed
    let mut interface_receivers: HashMap<IpAddr, ActoRef<()>> = HashMap::new();

    // Start receivers for initial interface sockets
    let initial_interfaces = sockets2.get_all_interface_addresses_v4();
    for addr in initial_interfaces {
        if let Some(socket) = sockets2.get_interface_socket_v4(addr) {
            let service_name = service_name.clone();
            let snd_ref = snd_ref.clone();
            let addr_str = addr.to_string();
            let receiver_ref = ctx
                .spawn_supervised(&format!("receiver_interface_{}", addr_str), move |ctx| {
                    receiver(ctx, service_name, socket, snd_ref)
                });
            interface_receivers.insert(IpAddr::V4(addr), receiver_ref);
            tracing::info!("Started receiver for initial interface {}", addr);
        }
    }

    // only stop when a supervised actor stops
    loop {
        let msg = ctx.recv().await;
        match msg {
            ActoInput::NoMoreSenders => {}
            ActoInput::Supervision { id, name, result } => {
                match result {
                    Ok(Ok(_)) => tracing::warn!("actor {:?} ({}) stopped", id, name),
                    Ok(Err(e)) => {
                        tracing::warn!("actor {:?} ({}) failed: {}", id, name, e)
                    }
                    Err(e) => {
                        tracing::warn!("actor {:?} ({}) aborted: {}", id, name, e);
                    }
                }
                break;
            }
            ActoInput::Message(msg) => match &msg {
                Input::AddInterface(addr) => {
                    if let IpAddr::V4(ipv4) = addr {
                        if let Err(e) = sockets2.add_interface_v4(*ipv4) {
                            tracing::warn!("Failed to add interface {}: {}", addr, e);
                        } else {
                            // Start a receiver for the new interface socket
                            if let Some(socket) = sockets2.get_interface_socket_v4(*ipv4) {
                                let service_name = service_name.clone();
                                let snd_ref = snd_ref.clone();
                                let addr_str = addr.to_string();
                                let receiver_ref = ctx.spawn_supervised(
                                    &format!("receiver_interface_{}", addr_str),
                                    move |ctx| receiver(ctx, service_name, socket, snd_ref),
                                );
                                interface_receivers.insert(*addr, receiver_ref);
                                tracing::info!("Started receiver for interface {}", addr);
                            }
                        }
                    }
                }
                Input::RemoveInterface(addr) => {
                    if let IpAddr::V4(ipv4) = addr {
                        sockets2.remove_interface_v4(*ipv4);
                        // Remove the receiver reference for this interface
                        if interface_receivers.remove(addr).is_some() {
                            tracing::info!("Removed receiver reference for interface {}", addr);
                        }
                    }
                }
                _ => {
                    snd_ref.send(sender::MdnsMsg::Update(msg));
                }
            },
        }
    }
}
