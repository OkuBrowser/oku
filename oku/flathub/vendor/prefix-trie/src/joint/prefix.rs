use crate::Prefix;
use either::{Either, Left, Right};

/// Trait that defines a joint prefix, for instance, one that describes either a IPv4 or IPv6
/// address.
///
/// The joint prefix is implemented for [`ipnet::IpNet`], [`ipnetwork::IpNetwork`], and
/// [`cidr::IpCidr`]. It is also implemented for [`either::Either`], for which `P1` and `P2` both
/// implement [`crate::Prefix`].
pub trait JointPrefix {
    /// The first prefix type, e.g., IPv4
    type P1: Prefix + Clone;
    /// The second prefix type, e.g., IPv6
    type P2: Prefix + Clone;

    /// Get either `Left(P1)` or `Right(P2)`.
    fn p1_or_p2(self) -> Either<Self::P1, Self::P2>;

    /// Get either `Left(P1)` or `Right(P2)`, as a reference.
    fn p1_or_p2_ref(&self) -> Either<&Self::P1, &Self::P2>;

    /// Construct a prefix from a reference to the first variant.
    fn from_p1(p: &Self::P1) -> Self;

    /// Construct a prefix from a reference to the second variant.
    fn from_p2(p: &Self::P2) -> Self;
}

impl<P1, P2> JointPrefix for Either<P1, P2>
where
    P1: Prefix + Clone,
    P2: Prefix + Clone,
{
    type P1 = P1;
    type P2 = P2;

    fn p1_or_p2(self) -> Either<Self::P1, Self::P2> {
        self
    }

    fn p1_or_p2_ref(&self) -> Either<&Self::P1, &Self::P2> {
        self.as_ref()
    }

    fn from_p1(p: &Self::P1) -> Self {
        Left(p.clone())
    }

    fn from_p2(p: &Self::P2) -> Self {
        Right(p.clone())
    }
}

#[cfg(feature = "ipnet")]
impl JointPrefix for ipnet::IpNet {
    type P1 = ipnet::Ipv4Net;
    type P2 = ipnet::Ipv6Net;

    fn p1_or_p2(self) -> Either<ipnet::Ipv4Net, ipnet::Ipv6Net> {
        match self {
            ipnet::IpNet::V4(p) => Left(p),
            ipnet::IpNet::V6(p) => Right(p),
        }
    }

    fn p1_or_p2_ref(&self) -> Either<&ipnet::Ipv4Net, &ipnet::Ipv6Net> {
        match self {
            ipnet::IpNet::V4(p) => Left(p),
            ipnet::IpNet::V6(p) => Right(p),
        }
    }

    fn from_p1(p: &Self::P1) -> Self {
        ipnet::IpNet::V4(*p)
    }

    fn from_p2(p: &Self::P2) -> Self {
        ipnet::IpNet::V6(*p)
    }
}

#[cfg(feature = "ipnetwork")]
impl JointPrefix for ipnetwork::IpNetwork {
    type P1 = ipnetwork::Ipv4Network;
    type P2 = ipnetwork::Ipv6Network;

    fn p1_or_p2(self) -> Either<ipnetwork::Ipv4Network, ipnetwork::Ipv6Network> {
        match self {
            ipnetwork::IpNetwork::V4(p) => Left(p),
            ipnetwork::IpNetwork::V6(p) => Right(p),
        }
    }

    fn p1_or_p2_ref(&self) -> Either<&ipnetwork::Ipv4Network, &ipnetwork::Ipv6Network> {
        match self {
            ipnetwork::IpNetwork::V4(p) => Left(p),
            ipnetwork::IpNetwork::V6(p) => Right(p),
        }
    }

    fn from_p1(p: &Self::P1) -> Self {
        ipnetwork::IpNetwork::V4(*p)
    }

    fn from_p2(p: &Self::P2) -> Self {
        ipnetwork::IpNetwork::V6(*p)
    }
}

#[cfg(feature = "cidr")]
impl JointPrefix for cidr::IpCidr {
    type P1 = cidr::Ipv4Cidr;
    type P2 = cidr::Ipv6Cidr;

    fn p1_or_p2(self) -> Either<cidr::Ipv4Cidr, cidr::Ipv6Cidr> {
        match self {
            cidr::IpCidr::V4(p) => Left(p),
            cidr::IpCidr::V6(p) => Right(p),
        }
    }

    fn p1_or_p2_ref(&self) -> Either<&cidr::Ipv4Cidr, &cidr::Ipv6Cidr> {
        match self {
            cidr::IpCidr::V4(p) => Left(p),
            cidr::IpCidr::V6(p) => Right(p),
        }
    }

    fn from_p1(p: &Self::P1) -> Self {
        cidr::IpCidr::V4(*p)
    }

    fn from_p2(p: &Self::P2) -> Self {
        cidr::IpCidr::V6(*p)
    }
}
