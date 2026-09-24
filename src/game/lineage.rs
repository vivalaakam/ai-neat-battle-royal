use vivalaakam_neuro_neat::Organism;

pub fn parse_serial(id: &str) -> Option<u64> {
    id.parse().ok()
}

pub fn organism_serial(organism: &Organism) -> Option<u64> {
    organism.get_id().and_then(|id| parse_serial(id))
}

/// Assigns the next sequential lineage number and stores it on the organism.
pub fn assign_serial(organism: &mut Organism, next: &mut u64) -> u64 {
    let serial = *next;
    *next += 1;
    organism.set_id(serial.to_string());
    serial
}

pub fn tag_if_missing(organism: &mut Organism, next: &mut u64) {
    if organism.get_id().is_none() {
        assign_serial(organism, next);
    }
}
