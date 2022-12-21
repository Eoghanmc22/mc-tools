#![feature(split_array)]
#![deny(meta_variable_misuse)]

use std::fmt::Debug;

pub mod packets;
pub mod primitive;

pub trait Data<'a>: Sized {
    fn try_decode(buffer: &mut &'a [u8]) -> Result<Self, DecodingError>;

    fn expected_size(&self) -> usize;
    // TODO Can this be implemented without returning a buffer? ie fn(&self, buffer: &mut &'b mut [u8])
    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8];
}

pub trait Packet<'a>: Data<'a> + Debug {
    type Proto: Debug;
    const PACKET_ID: Self::Proto;
    const PACKET_ID_NUM: u8;
    const DIRECTION: Direction;
}

pub enum Direction {
    ServerToClient,
    ClientToServer,
}

#[derive(thiserror::Error, Debug)]
pub enum DecodingError {
    #[error("Incomplete buffer")]
    EOF,
    #[error("Buffer contained invalid data")]
    BadData,
    #[error("The packet reader for {0} did not read the entire packet")]
    DirtyBuffer(String),
}

// Mostly taken from Graphite
#[macro_export]
macro_rules! define_proto {
    ($proto_name:ident, $proto_id:expr, $dir:expr => { $( $packet:ident $(<$life:lifetime>)? = $packet_id:expr),* }) => {
        #[derive(Debug, Eq, PartialEq)]
        #[repr(u8)]
        pub enum $proto_name {
            $( $packet = $packet_id, )*
        }

        $(
            impl<'a> $crate::Packet<'a> for $packet $( <$life> )? {
                type Proto = $proto_name;
                const PACKET_ID: $proto_name = $proto_name::$packet;
                const PACKET_ID_NUM: u8 = $packet_id;
                const DIRECTION: $crate::Direction = $dir;
            }
        )*

        impl $proto_name {
            pub const PROTOCOL_ID: u8 = $proto_id;
        }

        paste::paste! {
            pub trait [< PacketHandler $proto_name >] {
                type Error: std::error::Error + From<$crate::DecodingError>;

                $(
                    fn [<handle_ $packet:snake>](&mut self, _: $packet) -> Result<(), Self::Error> {
                        Ok(())
                    }
                )*

                fn [< parse_and_handle_ $proto_name:snake >] <'a, I: Into<&'a [u8]>> (&mut self, packet: I) -> Result<(), Self::Error> {
                    let mut bytes = packet.into();

                    let packet_id: u8 = $crate::Data::try_decode(&mut bytes)?;
                    match packet_id {
                        $(
                            <$packet as $crate::Packet>::PACKET_ID_NUM => {
                                let packet = <$packet as $crate::Data>::try_decode(&mut bytes)?;

                                if !bytes.is_empty() {
                                    return Err($crate::DecodingError::DirtyBuffer(format!("{:?}", packet_id)).into())?
                                }

                                Ok(self.[<handle_ $packet:snake>](packet)?)
                            }
                        )*
                        _ => {
                            // Ignore unknown packets
                            Ok(( ))
                        }
                    }
                }
            }
        }
    };
}

// TODO Add tests?
// Fully qualify types?
#[macro_export]
macro_rules! define_data {
    (pub struct $packet:ident $( < $life:lifetime > )? { $( pub $field_name:ident : $field_type:ty $( as $field_net_type:ty )? ),* }) => {
        #[derive(Clone, PartialEq, Debug)]
        pub struct $packet $( < $life > )? {
            $(
                pub $field_name : $field_type,
            )*
        }

        impl<'a> $crate::Data<'a> for $packet < $( $life )? > {
            fn try_decode(buffer: &mut &'a [u8]) -> Result<Self, $crate::DecodingError> {
                Ok($packet {
                    $(
                        $field_name : $crate::impl_decode!($( from $field_net_type => )? $field_name : $field_type, buffer),
                    )*
                })
            }

            fn expected_size(&self) -> usize {
                $(
                   $crate::impl_expected_size!($( from $field_net_type => )? $field_name : $field_type, self) +
                )* 0
            }

            fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
                $(
                    let buffer = $crate::impl_encode!($( from $field_net_type => )? $field_name : $field_type, self, buffer);
                )*

                buffer
            }
        }
    };
}

#[macro_export]
macro_rules! impl_decode {
    ($field_name:ident : $field_type:ty, $buffer:ident) => {
        <$field_type as $crate::Data>::try_decode($buffer)?
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $buffer:ident) => {
        <$field_net_type as $crate::Data>::try_decode($buffer)?.into()
    };
}

#[macro_export]
macro_rules! impl_expected_size {
    ($field_name:ident : $field_type:ty, $self:ident) => {
        <$field_type as $crate::Data>::expected_size(&$self.$field_name)
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $self:ident) => {
        <$field_net_type as $crate::Data>::expected_size(&$self.$field_name.into())
    };
}

#[macro_export]
macro_rules! impl_encode {
    ($field_name:ident : $field_type:ty, $self:ident, $buffer:ident) => {
        <$field_type as $crate::Data>::encode(&$self.$field_name, $buffer)
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $self:ident, $buffer:ident) => {
        <$field_net_type as $crate::Data>::encode(&$self.$field_name.into(), $buffer)
    };
}
