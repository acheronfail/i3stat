use std::{
    error::Error,
    fs::{
        read_dir,
        read_to_string,
    },
    path::PathBuf,
};

use super::{
    Item,
    ToItem,
};

struct Bat(PathBuf);

impl Bat {
    fn name(&self) -> String {
        self.0.file_name().unwrap().to_string_lossy().into_owned()
    }

    fn get_charge(&self) -> Result<f32, Box<dyn Error>> {
        macro_rules! get_usize {
            ($x: expr) => {
                read_to_string(self.0.join($x))?.trim().parse::<usize>()? as f32
            };
        }

        Ok(get_usize!("charge_now") / get_usize!("charge_full") * 100.0)
    }
}

pub struct Battery {
    batteries: Vec<Bat>,
}

impl Default for Battery {
    fn default() -> Self {
        let battery_dir = PathBuf::from("/sys/class/power_supply");
        let batteries = read_dir(&battery_dir)
            .unwrap()
            .into_iter()
            .filter_map(|res| {
                res.ok()
                    .and_then(|ent| match ent.file_type() {
                        Ok(ft) if ft.is_symlink() => Some(battery_dir.join(ent.file_name())),
                        _ => None,
                    })
                    .and_then(|dir| {
                        if dir.join("charge_now").exists() {
                            Some(Bat(dir))
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<_>>();

        Battery { batteries }
    }
}

impl ToItem for Battery {
    fn to_item(&self) -> Item {
        Item::new(
            self.batteries
                .iter()
                .map(|b| format!("{}:{:.0}%", b.name(), b.get_charge().unwrap()))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    fn update(&mut self, _sys: &mut sysinfo::System) {}
}
