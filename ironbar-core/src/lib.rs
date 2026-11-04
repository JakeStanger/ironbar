pub mod config;
pub mod error;

pub trait ToOptionsVec<const N: usize> {
    fn to_options_vec() -> [&'static str; N];
}
