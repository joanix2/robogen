use robogen_robotics::RobotModel;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhysicsError {
    #[error("physics backend unavailable: {0}")]
    Unavailable(String),
}

pub trait PhysicsBackend: Send {
    type World;
    fn build_world(&self, robot: &RobotModel) -> Result<Self::World, PhysicsError>;
    fn step(&mut self, world: &mut Self::World, dt_seconds: f32) -> Result<(), PhysicsError>;
}

pub struct UnavailablePhysics;
impl PhysicsBackend for UnavailablePhysics {
    type World = ();
    fn build_world(&self, _robot: &RobotModel) -> Result<Self::World, PhysicsError> {
        Err(PhysicsError::Unavailable(
            "Rapier integration is planned for milestone 5".into(),
        ))
    }
    fn step(&mut self, _world: &mut Self::World, _dt_seconds: f32) -> Result<(), PhysicsError> {
        Err(PhysicsError::Unavailable("no world was created".into()))
    }
}
