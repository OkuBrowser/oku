use super::*;
use crate::replica_item::ReplicaItem;
use crate::window_util::get_view_stack_page_by_name;
use crate::NODE;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;
use oku_core::fs::watch::ReplicaEvent;
use oku_core::iroh_docs::{CapabilityKind, NamespaceId};

pub fn replica_compare(o1: &glib::object::Object, o2: &glib::object::Object) -> std::cmp::Ordering {
    let o1 = o1.clone().downcast::<ReplicaItem>().ok();
    let o2 = o2.clone().downcast::<ReplicaItem>().ok();
    match (o1, o2) {
        (Some(o1), Some(o2)) => {
            // Home replicas come first, then writable replicas
            o2.home()
                .cmp(&o1.home())
                .then(o2.writable().cmp(&o1.writable()))
        }
        _ => std::cmp::Ordering::Equal,
    }
}

pub fn is_replica(o: &glib::object::Object, replica_id: &NamespaceId) -> bool {
    let o = o.clone().downcast::<ReplicaItem>().ok();
    let o_id = o.map(|x| x.id()).unwrap_or_default();
    let replica_id_str = oku_core::fs::util::fmt(replica_id);
    o_id == replica_id_str
}

impl Window {
    /// Mutate the replica list state based on what the Oku node emits.
    /// Returns if anything changed.
    pub fn handle_replica_event(&self, event: &ReplicaEvent) -> bool {
        let replicas_store = self.replicas_store();
        let old_store = replicas_store.snapshot();
        match *event {
            ReplicaEvent::Created(replica_id) => {
                replicas_store.insert_sorted(
                    &ReplicaItem::new(oku_core::fs::util::fmt(replica_id), true, false),
                    replica_compare,
                );
            }
            ReplicaEvent::Deleted(replica_id) => {
                replicas_store.retain(|x| !is_replica(x, &replica_id));
            }
            ReplicaEvent::Imported((replica_id, capability_kind, is_home_replica)) => {
                let replica_index =
                    replicas_store.find_with_equal_func(|o| is_replica(o, &replica_id));
                let replica_item = replica_index
                    .and_then(|x| {
                        replicas_store
                            .item(x)
                            .map(|y| y.downcast::<ReplicaItem>().ok())
                    })
                    .flatten();
                match replica_item {
                    None => {
                        replicas_store.insert_sorted(
                            &ReplicaItem::new(
                                oku_core::fs::util::fmt(replica_id),
                                matches!(capability_kind, CapabilityKind::Write),
                                is_home_replica,
                            ),
                            replica_compare,
                        );
                    }
                    Some(replica_item) => {
                        replica_item.set_properties(&[
                            ("id", &oku_core::fs::util::fmt(replica_id)),
                            (
                                "writable",
                                &matches!(capability_kind, CapabilityKind::Write),
                            ),
                            ("home", &is_home_replica),
                        ]);
                    }
                }
            }
            ReplicaEvent::Synced((replica_id, capability_kind, is_home_replica)) => {
                let replica_index =
                    replicas_store.find_with_equal_func(|o| is_replica(o, &replica_id));
                let replica_item = replica_index
                    .and_then(|x| {
                        replicas_store
                            .item(x)
                            .map(|y| y.downcast::<ReplicaItem>().ok())
                    })
                    .flatten();
                match replica_item {
                    None => {
                        replicas_store.insert_sorted(
                            &ReplicaItem::new(
                                oku_core::fs::util::fmt(replica_id),
                                matches!(capability_kind, CapabilityKind::Write),
                                is_home_replica,
                            ),
                            replica_compare,
                        );
                    }
                    Some(replica_item) => {
                        replica_item.set_properties(&[
                            ("id", &oku_core::fs::util::fmt(replica_id)),
                            (
                                "writable",
                                &matches!(capability_kind, CapabilityKind::Write),
                            ),
                            ("home", &is_home_replica),
                        ]);
                    }
                }
            }
            ReplicaEvent::Initialised => (),
        };
        self.imp().replicas_sidebar_initialised.get() && old_store != replicas_store.snapshot()
    }

    pub async fn setup_replicas(&self) {
        // Prevent interaction with replicas while we're setting up
        if let Some(replicas_page) =
            get_view_stack_page_by_name("replicas".to_string(), &self.imp().side_view_stack)
        {
            replicas_page.child().set_sensitive(false);
        }
        // Add all replicas
        if let Some(node) = NODE.get() {
            if let Ok(replicas) = node.list_replicas().await {
                let _home_replica = node.home_replica(&None).await; // To create the home replica if it doesn't exist yet
                let ctx = glib::MainContext::default();
                let this = self.clone();
                ctx.invoke(move || {
                    let replicas_store = this.replicas_store();
                    for (replica, capability_kind, is_home_replica) in replicas.iter() {
                        replicas_store.append(&ReplicaItem::new(
                            oku_core::fs::util::fmt(replica),
                            matches!(capability_kind, CapabilityKind::Write),
                            *is_home_replica,
                        ));
                    }
                });
            }
        }
        // Allow interaction again
        if let Some(replicas_page) =
            get_view_stack_page_by_name("replicas".to_string(), &self.imp().side_view_stack)
        {
            replicas_page.child().set_sensitive(true);
        }
    }
}
