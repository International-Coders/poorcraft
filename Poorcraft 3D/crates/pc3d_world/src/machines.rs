//! P3D-702: steam, electricity, heat, fluids, and typed machines.
//!
//! Carriers are explicit (`PowerType`); every machine declares its input
//! and output carrier and the network REFUSES to connect mismatched
//! ports — a mechanical shaft cannot charge a battery, it must pass
//! through a generator first. All arithmetic is integer milli-units and
//! every conversion loses energy (out ≤ in), so networks converge and
//! are deterministic. Water is a real consumable fluid: a boiler without
//! water produces nothing and burns no fuel.

/// Energy carriers, in conversion order: fuel burns to heat, heat boils
/// water to steam, steam drives a shaft (mechanical), a shaft turns a
/// generator (electrical).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerType {
    Heat,
    Steam,
    Mechanical,
    Electrical,
}

/// One fuel unit = 1_000 milli-fuel holding 8_000 milli-heat.
pub const FUEL_UNIT: i64 = 1_000;
/// Milli-heat released per milli-fuel burned (8 heat per fuel).
pub const HEAT_PER_FUEL: i64 = 8;
/// Boiler efficiency: steam = heat × 3/4.
pub const BOILER_NUM: i64 = 3;
pub const BOILER_DEN: i64 = 4;
/// Milli-fuel burned per tick at full burn.
pub const BURN_PER_TICK: i64 = 1_000;
/// Milli-water consumed per milli-steam produced (fluid coupling).
pub const WATER_PER_STEAM: i64 = 1;
/// Steam engine: mechanical = steam × 1/2, at most 2_000 milli-steam/tick.
pub const ENGINE_NUM: i64 = 1;
pub const ENGINE_DEN: i64 = 2;
pub const ENGINE_THROTTLE: i64 = 2_000;
/// Generator: electrical = mechanical × 9/10, at most 2_000/tick.
pub const GEN_NUM: i64 = 9;
pub const GEN_DEN: i64 = 10;
pub const GEN_THROTTLE: i64 = 2_000;
/// Battery capacity in milli-electrical; discharge yields ×4/5.
pub const BATTERY_CAP: i64 = 50_000;
pub const DIS_NUM: i64 = 4;
pub const DIS_DEN: i64 = 5;
/// Default wire/shaft capacity per edge per tick.
pub const EDGE_THROUGHPUT: i64 = 1_000;

/// A machine archetype with its typed ports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineKind {
    /// Fuel + water → steam.
    Boiler,
    /// Steam → mechanical shaft.
    SteamEngine,
    /// Mechanical shaft → electrical.
    Generator,
    /// Stores electrical; charges and discharges on the same carrier.
    Battery,
}

impl MachineKind {
    /// The carrier this machine consumes, if any.
    pub fn input_carrier(self) -> Option<PowerType> {
        match self {
            MachineKind::Boiler => None,
            MachineKind::SteamEngine => Some(PowerType::Steam),
            MachineKind::Generator => Some(PowerType::Mechanical),
            MachineKind::Battery => Some(PowerType::Electrical),
        }
    }

    /// The carrier this machine produces (Battery discharges Electrical).
    pub fn output_carrier(self) -> Option<PowerType> {
        match self {
            MachineKind::Boiler => Some(PowerType::Steam),
            MachineKind::SteamEngine => Some(PowerType::Mechanical),
            MachineKind::Generator => Some(PowerType::Electrical),
            MachineKind::Battery => Some(PowerType::Electrical),
        }
    }
}

/// One machine instance. Buffers are in the machine's carrier milli-units.
#[derive(Clone, Debug)]
pub struct Machine {
    pub id: u64,
    pub kind: MachineKind,
    pub on: bool,
    /// Milli-fuel (Boiler).
    pub fuel_milli: i64,
    /// Milli-water (Boiler): the fluid input.
    pub water_milli: i64,
    /// Input carrier milli-units awaiting conversion.
    pub in_buffer: i64,
    /// Converted output awaiting transfer downstream.
    pub out_buffer: i64,
    /// Battery charge (milli-electrical).
    pub stored: i64,
    /// Cumulative energy consumed/produced per carrier, for audits.
    pub lifetime_in: i64,
    pub lifetime_out: i64,
}

impl Machine {
    pub fn new(id: u64, kind: MachineKind) -> Machine {
        Machine {
            id,
            kind,
            on: true,
            fuel_milli: 0,
            water_milli: 0,
            in_buffer: 0,
            out_buffer: 0,
            stored: 0,
            lifetime_in: 0,
            lifetime_out: 0,
        }
    }

    /// One conversion pass for THIS machine only (fuel burn, steam
    /// production, shaft conversion, battery charging). Transfer along
    /// edges happens in `MachineNetwork::tick` after every machine ran.
    pub fn tick(&mut self) {
        match self.kind {
            MachineKind::Boiler => {
                if !self.on {
                    return;
                }
                let burn = BURN_PER_TICK.min(self.fuel_milli);
                let steam_full = burn * HEAT_PER_FUEL * BOILER_NUM / BOILER_DEN;
                // Water-limited: the fluid caps the boil, and only the
                // fuel fraction actually used is consumed.
                let steam = steam_full.min(self.water_milli);
                if steam <= 0 {
                    return;
                }
                // Round fuel UP so steam ≤ burned-heat × 3/4 holds even
                // when the fluid capped the boil (no conservation leak).
                let fuel_used = (burn * steam + steam_full - 1) / steam_full;
                let water_used = steam * WATER_PER_STEAM;
                self.fuel_milli -= fuel_used;
                self.water_milli -= water_used;
                self.out_buffer += steam;
                self.lifetime_in += fuel_used * HEAT_PER_FUEL;
                self.lifetime_out += steam;
            }
            MachineKind::SteamEngine | MachineKind::Generator => {
                if !self.on {
                    return;
                }
                let (num, den, throttle) = if self.kind == MachineKind::SteamEngine {
                    (ENGINE_NUM, ENGINE_DEN, ENGINE_THROTTLE)
                } else {
                    (GEN_NUM, GEN_DEN, GEN_THROTTLE)
                };
                let draw = throttle.min(self.in_buffer);
                if draw <= 0 {
                    return;
                }
                let produced = draw * num / den;
                self.in_buffer -= draw;
                self.out_buffer += produced;
                self.lifetime_in += draw;
                self.lifetime_out += produced;
            }
            MachineKind::Battery => {
                // Charge from whatever arrived on the wire.
                let room = BATTERY_CAP - self.stored;
                let take = self.in_buffer.min(room.max(0));
                self.in_buffer -= take;
                self.stored += take;
                self.lifetime_in += take;
                self.lifetime_out += take;
            }
        }
    }
}

/// A typed connection: `from`'s output carrier must equal `to`'s input
/// carrier. Battery sources discharge `stored` at DIS efficiency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wire {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub carrier: PowerType,
    /// Max milli-units per tick through this wire.
    pub throughput: i64,
}

/// Why a proposed connection was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    /// The producing machine has no output (impossible) or the types
    /// disagree — e.g. mechanical shaft straight into a battery.
    TypeMismatch,
    /// One endpoint does not exist.
    UnknownMachine,
    /// The exact wire is already installed.
    Duplicate,
}

#[derive(Clone, Debug, Default)]
pub struct MachineNetwork {
    pub machines: std::collections::BTreeMap<u64, Machine>,
    pub wires: std::collections::BTreeMap<u64, Wire>,
    next_machine: u64,
    next_wire: u64,
}

impl MachineNetwork {
    pub fn new() -> MachineNetwork {
        MachineNetwork::default()
    }

    pub fn add_machine(&mut self, kind: MachineKind) -> u64 {
        self.next_machine += 1;
        let id = self.next_machine;
        self.machines.insert(id, Machine::new(id, kind));
        id
    }

    /// Refuel / refill a boiler (milli-units).
    pub fn supply(&mut self, id: u64, fuel_milli: i64, water_milli: i64) -> bool {
        let Some(m) = self.machines.get_mut(&id) else {
            return false;
        };
        if m.kind != MachineKind::Boiler {
            return false;
        }
        m.fuel_milli += fuel_milli;
        m.water_milli += water_milli;
        true
    }

    pub fn set_on(&mut self, id: u64, on: bool) -> bool {
        let Some(m) = self.machines.get_mut(&id) else {
            return false;
        };
        m.on = on;
        true
    }

    /// Connect two machines; refuses type mismatches at build time so a
    /// mis-wired network can never exist.
    pub fn connect(&mut self, from: u64, to: u64) -> Result<u64, WireError> {
        let f = self.machines.get(&from).ok_or(WireError::UnknownMachine)?;
        let t = self.machines.get(&to).ok_or(WireError::UnknownMachine)?;
        let carrier = match (f.kind.output_carrier(), t.kind.input_carrier()) {
            (Some(a), Some(b)) if a == b => a,
            _ => return Err(WireError::TypeMismatch),
        };
        for w in self.wires.values() {
            if w.from == from && w.to == to {
                return Err(WireError::Duplicate);
            }
        }
        self.next_wire += 1;
        let id = self.next_wire;
        self.wires.insert(
            id,
            Wire {
                id,
                from,
                to,
                carrier,
                throughput: EDGE_THROUGHPUT,
            },
        );
        Ok(id)
    }

    /// One deterministic tick: every machine converts (id order), then
    /// every wire moves at most its throughput (id order). Battery
    /// sources discharge stored charge into the wire.
    pub fn tick(&mut self) {
        let ids: Vec<u64> = self.machines.keys().copied().collect();
        for id in ids {
            if let Some(m) = self.machines.get_mut(&id) {
                m.tick();
            }
        }
        let wire_ids: Vec<u64> = self.wires.keys().copied().collect();
        for wid in wire_ids {
            let (from, to, throughput, is_battery) = match self.wires.get(&wid) {
                Some(w) => {
                    let bat = self
                        .machines
                        .get(&w.from)
                        .map(|m| m.kind == MachineKind::Battery)
                        .unwrap_or(false);
                    (w.from, w.to, w.throughput, bat)
                }
                None => continue,
            };
            // Available at the source this tick.
            let available = {
                let src = &self.machines[&from];
                if is_battery {
                    src.stored * DIS_NUM / DIS_DEN
                } else {
                    src.out_buffer
                }
            };
            let room = self.machines[&to].in_buffer_max_free();
            let move_milli = throughput.min(available).min(room);
            if move_milli <= 0 {
                continue;
            }
            let src = self.machines.get_mut(&from).expect("checked above");
            if is_battery {
                // Discharge efficiency: the cell drains more than the
                // wire carries; the difference is lost, never created.
                let drained = move_milli * DIS_DEN / DIS_NUM;
                src.stored -= drained;
            } else {
                src.out_buffer -= move_milli;
            }
            let dst = self.machines.get_mut(&to).expect("checked above");
            dst.in_buffer += move_milli;
        }
    }

    /// Total electrical charge held by all batteries.
    pub fn stored_charge(&self) -> i64 {
        self.machines
            .values()
            .filter(|m| m.kind == MachineKind::Battery)
            .map(|m| m.stored)
            .sum()
    }
}

impl Machine {
    /// Free input space is unbounded (machines buffer whatever the wire
    /// delivers; throttles cap conversion, not intake).
    fn in_buffer_max_free(&self) -> i64 {
        i64::MAX / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typed ports are enforced at connect time: a shaft cannot charge
    /// a battery, nothing can feed a boiler, and the one true chain
    /// boiler→engine→generator→battery wires cleanly.
    #[test]
    fn p3d702_typed_connections_reject_mismatch() {
        let mut net = MachineNetwork::new();
        let boiler = net.add_machine(MachineKind::Boiler);
        let engine = net.add_machine(MachineKind::SteamEngine);
        let gen = net.add_machine(MachineKind::Generator);
        let bat = net.add_machine(MachineKind::Battery);

        // Correct chain: every edge type-checks.
        assert!(net.connect(boiler, engine).is_ok());
        assert!(net.connect(engine, gen).is_ok());
        assert!(net.connect(gen, bat).is_ok());
        // Type mismatches are refused before any state exists.
        assert_eq!(net.connect(engine, bat), Err(WireError::TypeMismatch));
        assert_eq!(net.connect(boiler, gen), Err(WireError::TypeMismatch));
        assert_eq!(net.connect(gen, boiler), Err(WireError::TypeMismatch));
        // A boiler has no input carrier at all.
        assert_eq!(MachineKind::Boiler.input_carrier(), None);
        // Unknown endpoints refused.
        assert_eq!(net.connect(999, bat), Err(WireError::UnknownMachine));
        // Duplicate wiring refused.
        assert_eq!(net.connect(boiler, engine), Err(WireError::Duplicate));
        // Carriers are ordered heat→steam→mech→electric; port decls agree.
        assert_eq!(MachineKind::Boiler.output_carrier(), Some(PowerType::Steam));
        assert_eq!(
            MachineKind::SteamEngine.input_carrier(),
            Some(PowerType::Steam)
        );
        assert_eq!(
            MachineKind::Generator.input_carrier(),
            Some(PowerType::Mechanical)
        );
        assert_eq!(
            MachineKind::Battery.input_carrier(),
            Some(PowerType::Electrical)
        );
    }

    /// The full fuel→steam→shaft→electric chain charges a battery from
    /// nothing, every stage loses energy (out ≤ in), and the same
    /// construction runs bit-identically twice.
    #[test]
    fn p3d702_chain_charges_battery_with_losses() {
        let build = || {
            let mut net = MachineNetwork::new();
            let b = net.add_machine(MachineKind::Boiler);
            let e = net.add_machine(MachineKind::SteamEngine);
            let g = net.add_machine(MachineKind::Generator);
            let t = net.add_machine(MachineKind::Battery);
            net.connect(b, e).unwrap();
            net.connect(e, g).unwrap();
            net.connect(g, t).unwrap();
            assert!(net.supply(b, 10 * FUEL_UNIT, 20_000));
            (net, b, e, g, t)
        };
        let (mut net, b, e, g, t) = build();
        for _ in 0..40 {
            net.tick();
        }
        let stored = net.stored_charge();
        assert!(stored > 0, "fuel must reach the battery: {stored}");
        // Per-stage lifetime audits: each conversion lost energy.
        let m = |id: u64| &net.machines[&id];
        assert!(
            m(e).lifetime_out <= m(e).lifetime_in,
            "engine must lose energy"
        );
        assert!(
            m(g).lifetime_out * GEN_DEN <= m(g).lifetime_in * GEN_NUM,
            "generator at 9/10"
        );
        // Boiler: steam out ≤ heat potential of fuel burned (6 steam
        // milli per fuel milli = 8 heat × 3/4).
        assert!(
            m(b).lifetime_out * BOILER_DEN <= m(b).lifetime_in * BOILER_NUM,
            "boiler efficiency respected"
        );
        // Battery drained by the audit? No — nothing consumed it.
        assert_eq!(net.machines[&t].stored, stored);
        // Determinism: an identical build reaches the identical state.
        let (mut net2, ..) = build();
        for _ in 0..40 {
            net2.tick();
        }
        assert_eq!(net2.stored_charge(), stored);
        // Fuel and water were consumed.
        assert!(net.machines[&b].fuel_milli < 10 * FUEL_UNIT);
        assert!(net.machines[&b].water_milli < 20_000);
    }

    /// Water is a real fluid input: a dry boiler produces nothing and
    /// burns nothing; partial water caps steam to the available fluid.
    #[test]
    fn p3d702_water_starvation_caps_boiler() {
        let mut net = MachineNetwork::new();
        let boiler = net.add_machine(MachineKind::Boiler);
        let engine = net.add_machine(MachineKind::SteamEngine);
        net.connect(boiler, engine).unwrap();

        // Dry: no production, no fuel consumed.
        assert!(net.supply(boiler, 4 * FUEL_UNIT, 0));
        net.tick();
        assert_eq!(net.machines[&boiler].out_buffer, 0);
        assert_eq!(net.machines[&boiler].fuel_milli, 4 * FUEL_UNIT);

        // Water-limited: 500 milli-water yields at most 500 milli-steam
        // (WATER_PER_STEAM = 1) and refunds the unused fuel fraction.
        net.water_milli_at(boiler, 500);
        net.tick();
        // The wire moved the whole (tiny) boil downstream this tick.
        assert_eq!(
            net.machines[&engine].in_buffer, 500,
            "steam capped by fluid"
        );
        // Fuel burned only for the water-limited fraction, rounded up:
        // a full burn would boil 6_000, we boiled 500 → 84 milli-fuel.
        let fuel_left = net.machines[&boiler].fuel_milli;
        assert_eq!(fuel_left, 4 * FUEL_UNIT - 84, "fuel metered to output");
    }

    /// Switched-off machines convert nothing; fuel exhaustion stills
    /// the whole chain; wires cap throughput.
    #[test]
    fn p3d702_off_and_exhaustion_stop_the_chain() {
        let mut net = MachineNetwork::new();
        let boiler = net.add_machine(MachineKind::Boiler);
        let engine = net.add_machine(MachineKind::SteamEngine);
        net.connect(boiler, engine).unwrap();

        net.set_on(boiler, false);
        assert!(net.supply(boiler, FUEL_UNIT, 5_000));
        net.tick();
        assert_eq!(net.machines[&boiler].out_buffer, 0, "off boiler idles");
        assert_eq!(
            net.machines[&boiler].fuel_milli, FUEL_UNIT,
            "off boiler keeps fuel"
        );
        net.set_on(boiler, true);

        // With ample fuel+water the boiler boils 6_000/tick but the
        // wire only carries EDGE_THROUGHPUT downstream per tick, so the
        // surplus piles up in the boiler while the engine converts one
        // wire-load per tick (a documented one-tick pipeline lag).
        assert!(net.supply(boiler, 10 * FUEL_UNIT, 30_000));
        net.tick();
        assert_eq!(
            net.machines[&engine].in_buffer, EDGE_THROUGHPUT,
            "wire caps flow"
        );
        assert_eq!(net.machines[&boiler].out_buffer, 5_000, "surplus waits");
        assert_eq!(
            net.machines[&engine].lifetime_out, 0,
            "conversion lags one tick"
        );
        net.tick();
        assert_eq!(
            net.machines[&engine].in_buffer, EDGE_THROUGHPUT,
            "drained then refilled"
        );
        assert_eq!(
            net.machines[&boiler].out_buffer, 10_000,
            "+5_000/tick surplus"
        );
        assert_eq!(
            net.machines[&engine].lifetime_out, 500,
            "1_000 steam × 1/2 per tick"
        );

        // Exhaust the fuel: production stops for good.
        let mut net3 = MachineNetwork::new();
        let b = net3.add_machine(MachineKind::Boiler);
        let e = net3.add_machine(MachineKind::SteamEngine);
        net3.connect(b, e).unwrap();
        assert!(net3.supply(b, FUEL_UNIT, 10_000));
        for _ in 0..10 {
            net3.tick();
        }
        assert_eq!(net3.machines[&b].fuel_milli, 0, "fuel consumed");
        let steady = net3.machines[&b].out_buffer + net3.machines[&e].in_buffer;
        for _ in 0..5 {
            net3.tick();
        }
        assert_eq!(
            net3.machines[&b].out_buffer + net3.machines[&e].in_buffer,
            steady,
            "no production after exhaustion"
        );

        // Battery-only network still ticks (charges then holds).
        let mut net2 = MachineNetwork::new();
        let bat = net2.add_machine(MachineKind::Battery);
        net2.machines.get_mut(&bat).unwrap().in_buffer = BATTERY_CAP * 2;
        net2.tick();
        assert_eq!(net2.machines[&bat].stored, BATTERY_CAP, "capacity clamps");
        assert_eq!(net2.machines[&bat].in_buffer, BATTERY_CAP, "surplus waits");
    }
}

impl MachineNetwork {
    /// Test helper: set a boiler's water directly.
    fn water_milli_at(&mut self, id: u64, water_milli: i64) {
        if let Some(m) = self.machines.get_mut(&id) {
            m.water_milli = water_milli;
        }
    }
}
