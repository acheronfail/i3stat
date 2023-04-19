use sysinfo::{
    ComponentExt,
    System,
    SystemExt,
};

use super::{
    Item,
    ToItem,
};

// TODO: store list of references to Components, so don't have to iter?
pub struct Sensors {
    temp: f32,
}

impl Default for Sensors {
    fn default() -> Self {
        Sensors { temp: 0.0 }
    }
}

impl ToItem for Sensors {
    fn to_item(&self) -> Item {
        Item::new(format!("TMP: {:.0}°C", self.temp))
    }

    fn update(&mut self, sys: &mut System) {
        // TODO: support choosing particular one
        for c in sys.components_mut() {
            if c.label() == "coretemp Package id 0" {
                c.refresh();
                self.temp = c.temperature();
            }
        }
    }
}
