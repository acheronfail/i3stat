use sysinfo::{
    CpuExt,
    CpuRefreshKind,
    System,
    SystemExt,
};

use super::{
    Item,
    ToItem,
};

pub struct Cpu {
    pct: f32,
    precision: usize,
    zero_pad: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            pct: 0.0,
            precision: 0,
            zero_pad: true,
        }
    }
}

impl ToItem for Cpu {
    fn to_item(&self) -> Item {
        let pad = if !self.zero_pad {
            0
        } else if self.precision > 0 {
            // two digits + decimal separator + precision
            self.precision + 3
        } else {
            // two digits only
            2
        };

        Item::new(format!(
            "{:0pad$.precision$}%",
            self.pct,
            precision = self.precision,
            pad = pad
        ))
    }

    fn update(&mut self, sys: &mut System) {
        sys.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage());
        // dbg!(sys.load_average());
        // dbg!(sys.cpus());
        self.pct = sys.global_cpu_info().cpu_usage();
    }
}
