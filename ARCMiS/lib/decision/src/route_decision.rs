//! One routing decision: target role plus its basis.

use domain::Role;

use crate::route_basis::RouteBasis;

/// One routing decision: target role plus its basis.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteDecision {
    /// Specialist to dispatch.
    pub role: Role,
    /// How the router reached the decision.
    pub basis: RouteBasis,
}
