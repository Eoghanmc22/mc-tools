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

pub trait Packet: for<'a> Data<'a> {
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

macro_rules! define_packet {
    (pub struct $packet:ident $( < $life:lifetime > )? { $( $field_name:ident : $field_type:ty $( as $field_net_type:ty )? ),* }) => {
        pub struct $packet $( < $( $life , )* > )? {
            $( pub $field_name : $field_type )*
        }

        impl<'a> Data<'a> for $packet < $( $life )? > {
            fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError> {
                Ok($packet {
                    $( $field_name : {
                        let data $( : $field_net_type)? = Data::try_decode(buffer)?;
                        data.into()
                    } )*
                })
            }

            fn expected_size(&self) -> usize {
                fn convert_helper<T, U = T>(in: T) -> U { in.into() }

                1 + $( convert_helper $( ::<$field_type, $field_net_type> )? (self.$field_name).expected_size() + )* 0
            }

            fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8] {
                let buffer = var_int(<Self as Packet>::PACKET_ID).encode(buffer);

                $(
                    let buffer = convert_helper $( ::<$field_type, $field_net_type> )? (self.$field_name).encode(buffer);
                )*

                buffer
            }
        }
    };
}
