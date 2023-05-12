use sysinfo::{ComponentExt, RefreshKind, System, SystemExt};
fn main() {
    let sys = System::new_with_specifics(RefreshKind::new().with_components_list());
    sys.components()
        .into_iter()
        .for_each(|c| println!("{:>width$.2}°C:{}", c.temperature(), c.label(), width = 6));
}
