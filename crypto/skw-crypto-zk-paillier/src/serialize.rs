pub mod vecbigint {
    use std::fmt;

    use skw_crypto_curv::arithmetic::traits::*;
    use skw_crypto_curv::BigInt;

    use serde::de::SeqAccess;
    use serde::ser::SerializeSeq;
    use serde::{de, ser};

    pub fn serialize<S: ser::Serializer>(x: &[BigInt], serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(x.len()))?;
        for e in x {
            seq.serialize_element(&e.to_str_radix(10))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: de::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<BigInt>, D::Error> {
        struct VecBigIntVisitor;

        impl<'de> de::Visitor<'de> for VecBigIntVisitor {
            type Value = Vec<BigInt>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("vector of bigint")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Vec<BigInt>, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values: Vec<BigInt> = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    values.push(BigInt::from_str_radix(&value, 10).unwrap());
                }

                Ok(values)
            }
        }

        deserializer.deserialize_seq(VecBigIntVisitor)
    }
}
