use std::io::{Read, Write};
use euclid::default::*;
use mc_io::ConnectionContext;

pub struct Player<S: Read + Write> {
    pub entity_id: i32,
    pub proto_state: u8,
    pub username: String,

    pub connected: bool,
    pub teleported: bool,
    pub kicked: bool,

    pub position: Point3D<f64>,
    pub velocity: Point3D<f64>,

    pub connection: ConnectionContext<S>
}
