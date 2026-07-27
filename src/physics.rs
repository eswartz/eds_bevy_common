pub use avian3d::prelude::*;
pub use avian3d::math::*;
// pub use avian3d::PhysicsGizmos;
// pub use avian3d::PhysicsTime;
// pub use avian3d::PhysicsLayer;
// pub use avian3d::Physics;
// pub use avian3d::PhysicsTime as _;
pub use avian3d::dynamics::rigid_body::LinearVelocity;
pub use avian3d::dynamics::solver::SolverDiagnostics;
pub use avian3d::dynamics::solver::SolverConfig;

pub type Scalar = Real;
pub type Vector = RVector;
pub type Vector2 = RVec2;
pub type Vector3 = RVec3;
pub type Quaternion = bevy::math::Quat;

// pub type AdjustPrecision = ToRealPrecision;
pub use avian3d::math::ToF32Precision as _;
pub use avian3d::math::ToRealPrecision as _;
pub use avian3d::parry::glamx::approx::AbsDiffEq;
