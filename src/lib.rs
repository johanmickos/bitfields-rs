//! Helpers for grouping together data in sub-byte bitfields.
#![feature(try_from)]

use std::collections::HashMap;
use std::convert::TryFrom;
use std::mem;

pub type StorageType = u32;

#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooLarge,
    OutOfBounds,
    WouldOverlap,
    TryFromError,
}

#[derive(Debug)]
struct BitField {
    pos: StorageType,
    width: StorageType,
}
/// A set of bit fields
#[derive(Debug)]
pub struct BitFieldSet {
    /// Total number of bits spanned by this set
    num_bits: u32,
    storage: StorageType, // TODO support wider types
    entries: HashMap<u32, BitField>,
}

impl BitFieldSet {
    /// Creates a new [BitFieldSet] supporting at most `num_bits` internal bits.
    pub fn new(num_bits: u32) -> Result<Self, Error> {
        let supported_bits = (mem::size_of::<StorageType>() * 8) as u32;
        if num_bits > supported_bits {
            return Err(Error::OutOfBounds);
        }
        Ok(BitFieldSet {
            num_bits,
            storage: 0,
            entries: HashMap::new(),
        })
    }

    /// Creates an associative [BitField] entry in this [BitFieldSet]
    pub fn add(&mut self, pos: u32, width: u32) -> Result<(), Error> {
        // TODO leverage same OOB checks as `insert`
        if pos > self.num_bits {
            return Err(Error::OutOfBounds);
        }
        self.check_overflow(width, pos)?;
        if self.entries.contains_key(&pos) {
            return Err(Error::WouldOverlap);
        }
        self.entries.insert(pos, BitField { pos, width });
        Ok(())
    }

    /// Inserts the the data at the provided position and associates its position and width.
    pub fn insert<D: Into<StorageType>>(
        &mut self,
        pos: u32,
        width: u32,
        data: D,
    ) -> Result<StorageType, Error> {
        if pos > self.num_bits as StorageType {
            return Err(Error::OutOfBounds);
        }
        let data: StorageType = data.into();
        let data_too_large = (mem::size_of::<D>() as u32) > self.num_bits;
        self.check_overflow(pos, width)?;
        if data_too_large {
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

    pub fn get_raw(&self) -> StorageType {
        self.storage
    }

    fn check_overflow(&self, width: u32, pos: u32) -> Result<(), Error> {
        if (width + pos) > self.num_bits {
            Err(Error::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

impl From<StorageType> for BitFieldSet {
    fn from(raw: StorageType) -> Self {
        let supported_bits = (mem::size_of::<StorageType>() * 8) as u32;
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
    const RAW_STORAGE: StorageType = 0b10001001;

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
        let mut bfs = BitFieldSet::from(RAW_STORAGE);
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

    #[test]
    fn into_raw() {
        let mut bfs = BitFieldSet::new(8).unwrap();
        bfs.insert(PATH_TYPE_POS, 1, PathTypes::Unique as u8)
            .unwrap();
        bfs.insert(PROTOCOL_POS, 5, ProtocolTypes::UDP as u8)
            .unwrap();
        bfs.insert(ADDRESS_TYPE_POS, 2, AddressTypes::IPv6 as u8)
            .unwrap();
        let raw = bfs.get_raw();
        assert_eq!(
            raw, RAW_STORAGE,
            "Inserted data should match hardocded expected bits"
        );
    }
    #[test]
    fn truncated_raw() {
        let mut bfs = BitFieldSet::new(8).unwrap();
        bfs.insert(PATH_TYPE_POS, 1, PathTypes::Unique as u8)
            .unwrap();
        bfs.insert(PROTOCOL_POS, 5, ProtocolTypes::UDP as u8)
            .unwrap();
        bfs.insert(ADDRESS_TYPE_POS, 2, AddressTypes::IPv6 as u8)
            .unwrap();
        let raw = bfs.get_raw() as u8;
        assert_eq!(
            raw as u32, RAW_STORAGE,
            "Inserted data should match hardocded expected bits"
        );
    }

    #[test]
    fn bad_insertion() {
        let mut bfs = BitFieldSet::new(8).unwrap();
        // Valid insertion
        bfs.add(PATH_TYPE_POS, 1).unwrap();
        // Invalid re-insertion at existing position
        let res = bfs.add(PATH_TYPE_POS, 1);

        assert_eq!(res, Err(Error::WouldOverlap));
    }

    #[test]
    fn out_of_bounds_insertion() {
        let mut bfs = BitFieldSet::new(8).unwrap();
        let res = bfs.add(8, 9);
        assert_eq!(res, Err(Error::OutOfBounds));
    }
}
