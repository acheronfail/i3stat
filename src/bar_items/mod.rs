macro_rules! use_them {
    ($($mod:ident),*) => {
        $(
            pub mod $mod;
            pub use $mod::*;
        )*
    };
}

use_them!(battery, cpu, disk, dunst, kbd, krb, mem, net_usage, nic, pulse, script, sensors, time);
