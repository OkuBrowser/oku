use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    NoKnownPeers(#[from] libp2p::kad::NoKnownPeers),
    #[error(transparent)]
    Store(#[from] libp2p::kad::store::Error),
    #[error(transparent)]
    GetRecord(#[from] libp2p::kad::GetRecordError),
    #[error(transparent)]
    PutRecord(#[from] libp2p::kad::PutRecordError),
    #[error(transparent)]
    GetProviders(#[from] libp2p::kad::GetProvidersError),
    #[error(transparent)]
    AddProvider(#[from] libp2p::kad::AddProviderError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingOp {
    AddAddress,
    RemoveAddress,
    RemovePeer,
}
