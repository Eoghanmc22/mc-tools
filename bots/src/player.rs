use euclid::default::*;

pub struct Player {
    pub entity_id: i32,
    pub proto_state: u8,
    pub username: String,

    pub connected: bool,
    pub teleported: bool,
    pub kicked: bool,

    pub position: Point3D<f64>,
    pub velocity: Vector3D<f64>,
}
