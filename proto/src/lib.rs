#![feature(split_array)]

use std::fmt::Display;
pub mod packets;
pub mod primitive;

pub trait Data<'a>: Sized {
    fn try_decode<'b: 'a>(buffer: &'a mut &'b [u8]) -> Result<Self, DecodingError>;

    fn expected_size(&self) -> usize;
    fn encode<'b>(&self, buffer: &'b mut [u8]) -> &'b mut [u8];
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
