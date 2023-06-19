#[macro_export]
macro_rules! use_and_export {
    ($($mod:ident),*) => {
        $(
            pub mod $mod;
            pub use $mod::*;
        )*
    };
}

#[macro_export]
macro_rules! bail {
    ($arg:tt) => {
        return Err($arg.into())
    };
    ($($arg:tt)+) => {
        return Err(format!($($arg)*).into())
    };
}
