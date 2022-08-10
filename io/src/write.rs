use std::io::{Read, Write};
use crate::ctx::{ConnectionContext, GlobalContext};
use crate::error::CommunicationError;

pub fn write<S>(ctx: &mut GlobalContext, connection: &mut ConnectionContext<S>, buffer: &[u8]) -> Result<(), CommunicationError>
    where
        S: Read + Write,
{
    todo!()
}