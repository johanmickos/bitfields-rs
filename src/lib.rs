//! Helpers for grouping together data in sub-byte bitfields.
#![feature(try_from)]

use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;

type StorageType = u32;

#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooLarge,
    OutOfBounds,
    WouldOverlap,
    TryFromError,
}

struct BitField {
    pos: StorageType,
    width: StorageType,
}
/// A set of bit fields
pub struct BitFieldSet {
    /// Total number of bits spanned by this set
    num_bits: usize,
    storage: StorageType, // TODO support wider types
    entries: HashMap<StorageType, BitField>,
}

impl BitFieldSet {
    pub fn new(num_bits: usize) -> Result<Self, Error> {
        let supported_bits = mem::size_of::<StorageType>() * 8;
        if num_bits > supported_bits {
            return Err(Error::OutOfBounds);
        }
        Ok(BitFieldSet {
            num_bits: supported_bits,
            storage: 0,
            entries: HashMap::new(),
        })
    }

    /// Creates an associative [BitField] entry in this [BitFieldSet]
    pub fn add(&mut self, pos: StorageType, width: StorageType) -> Result<(), Error> {
        if pos > self.num_bits as StorageType {
            return Err(Error::OutOfBounds);
        }
        self.entries.insert(pos, BitField { pos, width });
        Ok(())
    }

    /// Inserts the the data at the provided position and associates its position and width.
    pub fn insert<D: Into<StorageType>>(
        &mut self,
        pos: StorageType,
        width: StorageType,
        data: D,
    ) -> Result<StorageType, Error> {
        if pos > self.num_bits as StorageType {
            return Err(Error::OutOfBounds);
        }
        let data: StorageType = data.into();
        let data_too_large = mem::size_of::<D>() > self.num_bits;
        let data_overflow = (width + pos) > self.num_bits as StorageType;
        if data_too_large || data_overflow {
            return Err(Error::DataTooLarge);
        }
        self.storage |= data << pos;
        self.entries.insert(pos, BitField { pos, width });
        Ok(data)
    }

    pub fn get(&self, pos: StorageType) -> Option<StorageType> {
        let entry = self.entries.get(&pos)?;
        let mask = (2 as StorageType).pow(entry.width) - 1;
        let mask = mask << entry.pos;
        let value = self.storage & mask;
        let value = value >> entry.pos;
        Some(value)
    }

    pub fn get_as<T: TryFrom<StorageType>>(&self, pos: StorageType) -> Result<T, Error> {
        let value = self.get(pos).ok_or_else(|| Error::TryFromError)?;
        T::try_from(value).map_err(|_| Error::TryFromError)
    }
}

impl From<StorageType> for BitFieldSet {
    fn from(raw: StorageType) -> Self {
        let supported_bits = mem::size_of::<StorageType>() * 8;
        BitFieldSet {
            num_bits: supported_bits,
            storage: raw,
            entries: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BitFieldSet, Error};
    use std::convert::TryFrom;
    use StorageType;

    const PATH_TYPE_POS: StorageType = 7;
    const PROTOCOL_POS: StorageType = 2;
    const ADDRESS_TYPE_POS: StorageType = 0;

    #[derive(Debug, PartialEq)]
    #[repr(u8)]
    enum PathTypes {
        Named,
        Unique,
    }

    #[derive(Debug, PartialEq)]
    #[repr(u8)]
    enum AddressTypes {
        IPv4,
        IPv6,
        Domain,
    }

    #[derive(Debug, PartialEq)]
    #[repr(u8)]
    enum ProtocolTypes {
        Local,
        TCP,
        UDP,
        UDT,
    }

    impl TryFrom<StorageType> for PathTypes {
        type Error = Error;

        fn try_from(value: StorageType) -> Result<Self, Self::Error> {
            match value {
                x if x == PathTypes::Named as StorageType => Ok(PathTypes::Named),
                x if x == PathTypes::Unique as StorageType => Ok(PathTypes::Unique),
                _other => Err(Error::TryFromError),
            }
        }
    }

    impl TryFrom<StorageType> for AddressTypes {
        type Error = Error;

        fn try_from(value: StorageType) -> Result<Self, Self::Error> {
            match value {
                x if x == AddressTypes::IPv4 as StorageType => Ok(AddressTypes::IPv4),
                x if x == AddressTypes::IPv6 as StorageType => Ok(AddressTypes::IPv6),
                x if x == AddressTypes::Domain as StorageType => Ok(AddressTypes::Domain),
                _other => Err(Error::TryFromError),
            }
        }
    }

    impl TryFrom<StorageType> for ProtocolTypes {
        type Error = Error;

        fn try_from(value: StorageType) -> Result<Self, Self::Error> {
            match value {
                x if x == ProtocolTypes::Local as StorageType => Ok(ProtocolTypes::Local),
                x if x == ProtocolTypes::TCP as StorageType => Ok(ProtocolTypes::TCP),
                x if x == ProtocolTypes::UDP as StorageType => Ok(ProtocolTypes::UDP),
                x if x == ProtocolTypes::UDT as StorageType => Ok(ProtocolTypes::UDT),
                _other => Err(Error::TryFromError),
            }
        }
    }

    #[test]
    fn insertion() {
        // TODO force compiler-aware mapping of position to type stored
        let mut bfs = BitFieldSet::new(8).expect("8 bits should fit into default storage type u32");
        bfs.insert(PATH_TYPE_POS, 1, PathTypes::Unique as u8)
            .expect("Data width of 1 should fit inside expected 32 bits");
        bfs.insert(PROTOCOL_POS, 5, ProtocolTypes::UDP as u8)
            .expect("Data width of 5 should fit inside expected 32 bits");
        bfs.insert(ADDRESS_TYPE_POS, 2, AddressTypes::IPv6 as u8)
            .expect("Data width of 2 should fit inside expected 32 bits");

        assert_eq!(
            bfs.get(PATH_TYPE_POS).unwrap(),
            PathTypes::Unique as StorageType
        );
        assert_eq!(
            bfs.get_as::<PathTypes>(PATH_TYPE_POS).unwrap(),
            PathTypes::Unique
        );
        assert_eq!(
            bfs.get_as::<AddressTypes>(ADDRESS_TYPE_POS).unwrap(),
            AddressTypes::IPv6
        );
        assert_eq!(
            bfs.get_as::<ProtocolTypes>(PROTOCOL_POS).unwrap(),
            ProtocolTypes::UDP
        );
    }

    #[test]
    fn from_raw() {
        let raw: StorageType = 0b10001001;
        let mut bfs = BitFieldSet::from(raw);
        bfs.add(PATH_TYPE_POS, 1)
            .expect("Data of width 1 should fit inside expected 32 bits");
        bfs.add(PROTOCOL_POS, 5)
            .expect("Data of width 1 should fit inside expected 32 bits");
        bfs.add(ADDRESS_TYPE_POS, 2)
            .expect("Data of width 1 should fit inside expected 32 bits");

        assert_eq!(
            bfs.get(PATH_TYPE_POS).unwrap(),
            PathTypes::Unique as StorageType
        );
        assert_eq!(
            bfs.get_as::<PathTypes>(PATH_TYPE_POS).unwrap(),
            PathTypes::Unique
        );
        assert_eq!(
            bfs.get_as::<AddressTypes>(ADDRESS_TYPE_POS).unwrap(),
            AddressTypes::IPv6
        );
        assert_eq!(
            bfs.get_as::<ProtocolTypes>(PROTOCOL_POS).unwrap(),
            ProtocolTypes::UDP
        );
    }
}
