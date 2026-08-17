pub mod ek1100;
pub mod el1002;
pub mod el1008;
pub mod el1124;
pub mod el1259;
pub mod el2002;
pub mod el2004;
pub mod el2008;
pub mod el2024;
pub mod el2521;
pub mod el2522;
pub mod el2634;
pub mod el2809;
pub mod el3001;
pub mod el3021;
pub mod el3024;
pub mod el3062_0030;
pub mod el3204;
pub mod el4002;
pub mod el4008;
pub mod el4732;
pub mod el5152;
pub mod el6021;
pub mod el7031;
pub mod el7031_0030;
pub mod el7041_0052;
pub(crate) mod el9505;
pub mod ep2339_0021;

// Re-export types that Beckhoff subdirectory devices access via `super::`
pub(crate) use crate::devices::{
    EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple,
};
