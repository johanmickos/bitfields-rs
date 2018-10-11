# bitfields-rs
Helpers for storing sub-byte enums in primitive types.

## Example

```rust

fn example() {
  // Could either be handwritten or extracted from a serialized network buffer
  let mut storage = vec![0u8];
  // write TCP bits
  storage.store(Transport::TCP).unwrap();
  // write address type bits
  storage.store(AddressType::IPv4).unwrap();
  
  // retrieve as named enums from positions indicated by `BitField` implementations
  assert_eq!(storage.get_as::<Transport>().unwrap(), Transport::TCP);
  assert_eq!(storage.get_as::<AddressType>().unwrap(), AddressType::IPv4);
}


#[derive(Debug, PartialEq)]
#[repr(u8)]
enum Transport {
  TCP = 0b01,
  UDP = 0b10,
  UDT = 0b11,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum AddressType {
    IPv4 = 0,
    IPv6 = 1,
    DomainName = 2,
}


impl BitField for Transport {
  const POS: usize = 0;
  const WIDTH: usize = 2;
}

impl BitField for AddressType {
    const POS: usize = 2;
    const WIDTH: usize = 2;
}

impl Into<u8> for AddressType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl Into<u8> for Transport {
  fn into(self) -> u8 {
    self as u8
  }
}

impl TryFrom<u8> for AddressType {
    type Error = SerError;

    fn try_from(x: u8) -> Result<Self, Self::Error> {
        match x {
            x if x == AddressType::IPv4 as u8 => Ok(AddressType::IPv4),
            x if x == AddressType::IPv6 as u8 => Ok(AddressType::IPv6),
            _ => Err(SerError::InvalidType("Unsupported AddressType".into())),
        }
    }
}

impl TryFrom<u8> for Transport {
  type Error = ();

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0b01 => Ok(Transport::TCP),
      0b10 => Ok(Transport::UDP),
      0b11 => Ok(Transport::UDT),
      _ => Err(()),
    }
  }
}
```
