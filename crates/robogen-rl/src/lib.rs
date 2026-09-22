use thiserror::Error;

pub trait Environment {
    type Observation;
    type Action;
    fn reset(&mut self, seed: u64) -> Self::Observation;
    fn step(&mut self, action: &Self::Action) -> Step<Self::Observation>;
}

pub struct Step<O> {
    pub observation: O,
    pub reward: f32,
    pub terminated: bool,
}

#[derive(Debug, Error)]
pub enum RlError {
    #[error("RL backend unavailable: PPO is planned for milestone 9")]
    Unavailable,
}

pub trait RlBackend {
    fn train(&mut self) -> Result<(), RlError>;
}
pub struct UnavailableRlBackend;
impl RlBackend for UnavailableRlBackend {
    fn train(&mut self) -> Result<(), RlError> {
        Err(RlError::Unavailable)
    }
}
