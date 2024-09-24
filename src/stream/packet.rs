use std::{any::Any, fmt::Debug, marker::PhantomData};

use bincode::ErrorKind;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};

mod actions;
mod packet_trait;

pub use actions::Action;
pub use packet_trait::PacketTrait;

use super::Bytes;
use crate::{time, NetError};

#[derive(Debug, Serialize)]
pub struct TransmissionPacket {
    timestamp: u128,
    pub action: Action,
    packet: Vec<u8>,
}
impl TransmissionPacket {
    /// Creates a new instance of `Packet` from the internally held bytes.
    pub fn new(packet_type: Action, packet: &[u8]) -> Self {
        Self {
            timestamp: time::get_unix_epoch_timestamp(),
            action: packet_type,
            packet: packet.to_vec(),
        }
    }

    pub fn get_packets(&self) -> &Vec<u8> {
        &self.packet
    }
}

impl Bytes for TransmissionPacket {
    fn from_bytes(bytes: &[u8]) -> Result<Self, NetError> {
        bincode::deserialize(bytes).map_err(NetError::BincodeError)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, NetError> {
        bincode::serialize(self).map_err(NetError::BincodeError)
    }
}

impl<'de> Deserialize<'de> for TransmissionPacket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Default)]
        struct TransmissionPacketVisitor {}

        impl<'de> Visitor<'de> for TransmissionPacketVisitor {
            type Value = TransmissionPacket;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a `TransmissionPacket` struct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut timestamp = None;
                let mut action = None;
                let mut packet = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "timestamp" => {
                            if timestamp.is_some() {
                                return Err(de::Error::duplicate_field("timestamp"));
                            };

                            timestamp = Some(map.next_value()?);
                        }
                        "action" => {
                            if action.is_some() {
                                return Err(de::Error::duplicate_field("action"));
                            };

                            action = Some(map.next_value()?);
                        }
                        "packet" => {
                            if packet.is_some() {
                                return Err(de::Error::duplicate_field("packet"));
                            };

                            packet = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                key,
                                &["timestamp", "action", "packet"],
                            ));
                        }
                    }
                }

                let timestamp = timestamp.ok_or_else(|| de::Error::missing_field("timestamp"))?;
                let action = action.ok_or_else(|| de::Error::missing_field("action"))?;
                let packet = packet.ok_or_else(|| de::Error::missing_field("packet"))?;

                Ok(TransmissionPacket {
                    timestamp,
                    action,
                    packet,
                })
            }
        }

        deserializer.deserialize_struct(
            "TransmissionPacket",
            &["timestamp", "action", "packet"],
            TransmissionPacketVisitor::default(),
        )
    }
}

/// Packet<S> is the deserialized data from the stream.
/// S is your deserialization enum.
#[derive(Debug, Serialize)]
pub struct Packet<S: Serialize + for<'a> Deserialize<'a> + PacketTrait> {
    timestamp: u128,
    pub packet_type: S,
    pub packet: Vec<u8>,
}
impl<S: Serialize + for<'a> Deserialize<'a> + PacketTrait> Packet<S> {
    pub fn new(packet_type: S, packet: Vec<u8>) -> Result<Self, Box<ErrorKind>> {
        Ok(Self {
            timestamp: time::get_unix_epoch_timestamp(),
            packet_type,
            packet,
        })
    }

    pub async fn unpack(self) -> Result<Box<dyn Any>, Box<ErrorKind>> {
        self.packet_type.to_struct(&self.packet)
    }
}

impl<'de, S: Serialize + for<'a> Deserialize<'a> + PacketTrait> Deserialize<'de> for Packet<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PacketVisitor<S> {
            marker: PhantomData<S>,
        }
        impl<S: Serialize + for<'a> Deserialize<'a>> Default for PacketVisitor<S> {
            fn default() -> Self {
                Self {
                    marker: Default::default(),
                }
            }
        }

        impl<'de, S: Serialize + for<'a> Deserialize<'a> + PacketTrait> Visitor<'de> for PacketVisitor<S> {
            type Value = Packet<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Packet struct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut timestamp = None;
                let mut packet_type = None;
                let mut packet = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "timestamp" => {
                            if timestamp.is_some() {
                                return Err(de::Error::duplicate_field("timestamp"));
                            };

                            timestamp = Some(map.next_value()?);
                        }
                        "packet_type" => {
                            if packet_type.is_some() {
                                return Err(de::Error::duplicate_field("packet_type"));
                            };

                            packet_type = Some(map.next_value()?);
                        }
                        "packet" => {
                            if packet.is_some() {
                                return Err(de::Error::duplicate_field("packet"));
                            };

                            packet = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(de::Error::unknown_field(
                                key,
                                &["timestamp", "packet_type", "packet"],
                            ))
                        }
                    }
                }

                let timestamp = timestamp.ok_or_else(|| de::Error::missing_field("timestamp"))?;
                let packet_type =
                    packet_type.ok_or_else(|| de::Error::missing_field("packet_type"))?;
                let packet = packet.ok_or_else(|| de::Error::missing_field("packet"))?;

                Ok(Packet {
                    timestamp,
                    packet_type,
                    packet,
                })
            }
        }

        deserializer.deserialize_struct(
            "Packet",
            &["timestamp", "packet_type", "packet"],
            PacketVisitor::default(),
        )
    }
}
