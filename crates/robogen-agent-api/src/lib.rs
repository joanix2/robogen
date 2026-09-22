//! Future AI boundary. No network or model implementation is included.

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssistantRequest {
    pub user_message: String,
    pub project_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssistantResponse {
    pub message: String,
    pub proposed_source: Option<String>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AssistantError {
    #[error("assistant IA non configuré")]
    Disabled,
}

pub trait AiAssistantProvider: Send + Sync {
    fn respond(&self, request: &AssistantRequest) -> Result<AssistantResponse, AssistantError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DisabledAiAssistant;
impl AiAssistantProvider for DisabledAiAssistant {
    fn respond(&self, _request: &AssistantRequest) -> Result<AssistantResponse, AssistantError> {
        Err(AssistantError::Disabled)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskSpec {
    pub observation_size: usize,
    pub action_size: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InitialPolicy {
    pub weights: Vec<f32>,
}

pub trait FoundationPolicyProvider: Send + Sync {
    fn initial_policy(&self, task: &TaskSpec) -> Option<InitialPolicy>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoFoundationPolicy;
impl FoundationPolicyProvider for NoFoundationPolicy {
    fn initial_policy(&self, _task: &TaskSpec) -> Option<InitialPolicy> {
        None
    }
}
