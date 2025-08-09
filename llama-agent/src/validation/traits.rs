//! Core validation traits and interfaces

use crate::types::Session;

/// Core validation trait that all validators implement
pub trait Validator<Target> {
    type Error;

    fn validate(&self, session: &Session, target: &Target) -> Result<(), Self::Error>;
}
