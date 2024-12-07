use core::{fmt, marker::PhantomData};

use super::{DesIntoWithConfig, DeserializeInto, DeserializerConfig, MaybeDesIntoWithConfig};

pub struct MapDeserializer<KD, VD>(PhantomData<(KD, VD)>);

#[cfg(feature = "std")]
impl<K, V, KD, VD> DeserializeInto<std::collections::HashMap<K, V>> for MapDeserializer<KD, VD>
where
    K: Eq + core::hash::Hash,
    KD: DeserializeInto<K>,
    VD: DeserializeInto<V>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<std::collections::HashMap<K, V>, D::Error> {
        struct Visitor<'c, K, V, KD, VD>(&'c DeserializerConfig, PhantomData<(K, V, KD, VD)>);

        impl<'de, K, V, KD, VD> serde::de::Visitor<'de> for Visitor<'_, K, V, KD, VD>
        where
            K: Eq + core::hash::Hash,
            KD: DeserializeInto<K>,
            VD: DeserializeInto<V>,
        {
            type Value = std::collections::HashMap<K, V>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let capacity = super::size_hint::cautious::<(K, V)>(map.size_hint());
                let mut inner = std::collections::HashMap::with_capacity(capacity);

                while let Some(key) = map.next_key_seed(DesIntoWithConfig::<KD, K>::new(self.0))? {
                    let val = map.next_value_seed(MaybeDesIntoWithConfig::<VD, V>::new(self.0))?;
                    let Some(val) = val.unwrap_for_omittable::<A::Error>(self.0, "in map")? else {
                        continue;
                    };
                    inner.insert(key, val);
                }

                Ok(inner)
            }
        }

        deserializer.deserialize_map(Visitor::<K, V, KD, VD>(config, PhantomData))
    }
}

impl<K, V, KD, VD> DeserializeInto<alloc::collections::BTreeMap<K, V>> for MapDeserializer<KD, VD>
where
    K: Ord,
    KD: DeserializeInto<K>,
    VD: DeserializeInto<V>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<alloc::collections::BTreeMap<K, V>, D::Error> {
        struct Visitor<'c, K, V, KD, VD>(&'c DeserializerConfig, PhantomData<(K, V, KD, VD)>);

        impl<'de, K, V, KD, VD> serde::de::Visitor<'de> for Visitor<'_, K, V, KD, VD>
        where
            K: Ord,
            KD: DeserializeInto<K>,
            VD: DeserializeInto<V>,
        {
            type Value = alloc::collections::BTreeMap<K, V>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut inner = alloc::collections::BTreeMap::new();

                while let Some(key) = map.next_key_seed(DesIntoWithConfig::<KD, K>::new(self.0))? {
                    let val = map.next_value_seed(MaybeDesIntoWithConfig::<VD, V>::new(self.0))?;
                    let Some(val) = val.unwrap_for_omittable::<A::Error>(self.0, "in map")? else {
                        continue;
                    };
                    inner.insert(key, val);
                }

                Ok(inner)
            }
        }

        deserializer.deserialize_map(Visitor::<K, V, KD, VD>(config, PhantomData))
    }
}
