#![feature(split_array)]
#![deny(meta_variable_misuse)]

use std::fmt::Display;

pub mod packets;
pub mod primitive;

pub trait Data<'a>: Sized {
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError>;

    fn expected_size(&self) -> usize;
    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8];
}

pub trait Packet<'a>: Data<'a> {
    const PACKET_ID: u8;
    const DIRECTION: Direction;
}

pub enum Direction {
    ServerToClient,
    ClientToServer,
}

#[derive(thiserror::Error, Debug)]
pub enum DecodingError {
    EOF,
    BadData,
}

impl Display for DecodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// TODO Add tests?
#[macro_export]
macro_rules! define_packet {
    (pub struct $packet:ident $( < $life:lifetime > )? { $( pub $field_name:ident : $field_type:ty $( as $field_net_type:ty )? ),* }) => {
        pub struct $packet $( < $life > )? {
            $(
                pub $field_name : $field_type,
            )*
        }

        impl<'a> Data<'a> for $packet < $( $life )? > {
            fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
                Ok($packet {
                    $(
                        $field_name : $crate::impl_decode!($( from $field_net_type => )? $field_name : $field_type, buffer),
                    )*
                })
            }

            fn expected_size(&self) -> usize {
                1 +
                    $(
                        $crate::impl_expected_size!($( from $field_net_type => )? $field_name : $field_type, self) +
                    )*
                0
            }

            fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
                let buffer = var_int(<Self as Packet>::PACKET_ID as i32).encode(buffer);

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
        <$field_type as Data>::try_decode($buffer)?
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $buffer:ident) => {
        <$field_net_type as Data>::try_decode($buffer)?.into()
    };
}

#[macro_export]
macro_rules! impl_expected_size {
    ($field_name:ident : $field_type:ty, $self:ident) => {
        <$field_type as Data>::expected_size(&$self.$field_name)
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $self:ident) => {
        <$field_net_type as Data>::expected_size(&$self.$field_name.into())
    };
}

#[macro_export]
macro_rules! impl_encode {
    ($field_name:ident : $field_type:ty, $self:ident, $buffer:ident) => {
        <$field_type as Data>::encode(&$self.$field_name, $buffer)
    };
    (from $field_net_type:ty => $field_name:ident : $field_type:ty, $self:ident, $buffer:ident) => {
        <$field_net_type as Data>::encode(&$self.$field_name.into(), $buffer)
    };
}
