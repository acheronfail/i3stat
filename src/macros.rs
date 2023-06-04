#[macro_export]
macro_rules! use_and_export {
    ($($mod:ident),*) => {
        $(
            pub mod $mod;
            pub use $mod::*;
        )*
    };
}
