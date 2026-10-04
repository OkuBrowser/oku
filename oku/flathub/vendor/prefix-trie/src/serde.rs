//! Serialization and Deserialization implementation

use ::serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::*;

impl<P: Prefix + Serialize, T: Serialize> Serialize for PrefixMap<P, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let map: Vec<(&P, &T)> = Vec::from_iter(self);
        map.serialize(serializer)
    }
}

impl<P: Prefix + Serialize> Serialize for PrefixSet<P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let set: Vec<&P> = Vec::from_iter(self);
        set.serialize(serializer)
    }
}

impl<'de, P: Prefix + Deserialize<'de>, T: Deserialize<'de>> Deserialize<'de> for PrefixMap<P, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: Vec<(P, T)> = Vec::deserialize(deserializer)?;
        Ok(Self::from_iter(map))
    }
}

impl<'de, P: Prefix + Deserialize<'de>> Deserialize<'de> for PrefixSet<P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let set: Vec<P> = Vec::deserialize(deserializer)?;
        Ok(Self::from_iter(set))
    }
}

#[cfg(test)]
#[cfg(feature = "ipnet")]
mod test {
    use super::*;
    use num_traits::{NumCast, PrimInt};

    fn ip<P: Prefix>(s: &str) -> P {
        let ip: ipnet::Ipv4Net = s.parse().unwrap();
        let r = ip.addr().to_bits();
        let len = ip.prefix_len();

        let type_len = P::zero().repr().count_zeros() as usize;
        assert!(type_len >= 32);

        let r: <P as Prefix>::R =
            <<P as Prefix>::R as NumCast>::from(r).unwrap() << (type_len - 32);
        P::from_repr_len(r, len)
    }

    #[generic_tests::define]
    mod t {
        use super::*;
        use std::fmt::Debug;

        #[test]
        fn map<P: Prefix + Debug + PartialEq + Serialize + for<'de> Deserialize<'de>>() {
            let map: PrefixMap<P, usize> = PrefixMap::from_iter([
                (ip("10.0.0.0/8"), 1),
                (ip("10.1.0.0/16"), 2),
                (ip("10.2.0.0/16"), 3),
                (ip("10.2.1.1/24"), 4),
                (ip("10.2.2.2/24"), 5),
            ]);
            let s: String = serde_json::to_string(&map).unwrap();
            let map2: PrefixMap<P, usize> = serde_json::from_str(&s).unwrap();
            assert_eq!(map, map2);
        }

        #[test]
        fn set<P: Prefix + Debug + PartialEq + Serialize + for<'de> Deserialize<'de>>() {
            let set: PrefixSet<P> = PrefixSet::from_iter([
                ip("10.0.0.0/8"),
                ip("10.1.0.0/16"),
                ip("10.2.0.0/16"),
                ip("10.2.1.1/24"),
                ip("10.2.2.2/24"),
            ]);
            let s: String = serde_json::to_string(&set).unwrap();
            let set2: PrefixSet<P> = serde_json::from_str(&s).unwrap();
            assert_eq!(set, set2);
        }

        #[instantiate_tests(<(u32, u8)>)]
        mod raw32 {}

        #[instantiate_tests(<(u64, u8)>)]
        mod raw64 {}

        #[instantiate_tests(<(u128, u8)>)]
        mod raw128 {}

        #[cfg(feature = "ipnet")]
        #[instantiate_tests(<ipnet::Ipv4Net>)]
        mod ipv4net {}

        #[cfg(feature = "ipnet")]
        #[instantiate_tests(<ipnet::Ipv6Net>)]
        mod ipv6net {}

        #[cfg(feature = "ipnetwork")]
        #[instantiate_tests(<ipnetwork::Ipv4Network>)]
        mod ipv4network {}

        #[cfg(feature = "ipnetwork")]
        #[instantiate_tests(<ipnetwork::Ipv6Network>)]
        mod ipv6network {}

        #[cfg(feature = "cidr")]
        #[instantiate_tests(<cidr::Ipv4Cidr>)]
        mod ipv4cidr {}

        #[cfg(feature = "cidr")]
        #[instantiate_tests(<cidr::Ipv6Cidr>)]
        mod ipv6cidr {}

        #[cfg(feature = "cidr")]
        #[instantiate_tests(<cidr::Ipv4Inet>)]
        mod ipv4inet {}

        #[cfg(feature = "cidr")]
        #[instantiate_tests(<cidr::Ipv6Inet>)]
        mod ipv6inet {}
    }
}
