//! P3D-701: valve-era computing — signals, logic gates, circuits.
//!
//! Deterministic logic components on the D-012 valve aesthetic. Signals
//! are u8 (0 = off, >0 = on with strength). Gates evaluate pure Boolean
//! logic. Circuits compose gates into multi-stage computations with
//! deterministic evaluation order.

/// A signal: 0 (off) or >0 (on, value = strength).
pub type Signal = u8;

/// Logic gate kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GateKind {
    And,
    Or,
    Not,
    Xor,
}

impl GateKind {
    /// Evaluate a gate with 1-2 input signals.
    pub fn evaluate(&self, inputs: &[Signal]) -> Signal {
        match self {
            GateKind::And => {
                if inputs.iter().all(|&s| s > 0) {
                    inputs.iter().copied().min().unwrap_or(0)
                } else {
                    0
                }
            }
            GateKind::Or => inputs.iter().copied().max().unwrap_or(0),
            GateKind::Not => {
                if inputs.first().map_or(false, |&s| s > 0) {
                    0
                } else {
                    255
                }
            }
            GateKind::Xor => {
                let high = inputs.iter().filter(|&&s| s > 0).count();
                if high % 2 == 1 {
                    inputs.iter().copied().max().unwrap_or(0)
                } else {
                    0
                }
            }
        }
    }
}

/// A named gate in a circuit.
#[derive(Clone, Debug)]
pub struct LogicGate {
    pub name: String,
    pub kind: GateKind,
    /// Indices of input signals (into the circuit's signal array).
    pub inputs: Vec<usize>,
    /// Index of the output signal.
    pub output: usize,
}

/// A logic circuit: signals + gates, evaluated in declaration order.
#[derive(Clone, Debug, Default)]
pub struct LogicCircuit {
    pub signals: Vec<Signal>,
    pub gates: Vec<LogicGate>,
}

impl LogicCircuit {
    pub fn new(signal_count: usize) -> Self {
        LogicCircuit {
            signals: vec![0; signal_count],
            gates: Vec::new(),
        }
    }

    pub fn set_input(&mut self, idx: usize, value: Signal) {
        if idx < self.signals.len() {
            self.signals[idx] = value;
        }
    }

    pub fn add_gate(&mut self, name: &str, kind: GateKind, inputs: Vec<usize>, output: usize) {
        self.gates.push(LogicGate {
            name: name.to_string(),
            kind,
            inputs,
            output,
        });
    }

    /// Evaluate all gates in declaration order (deterministic).
    pub fn evaluate(&mut self) {
        for gate in &self.gates {
            let inputs: Vec<Signal> = gate
                .inputs
                .iter()
                .map(|&i| self.signals.get(i).copied().unwrap_or(0))
                .collect();
            let result = gate.kind.evaluate(&inputs);
            if gate.output < self.signals.len() {
                self.signals[gate.output] = result;
            }
        }
    }

    /// Get a signal value.
    pub fn get(&self, idx: usize) -> Signal {
        self.signals.get(idx).copied().unwrap_or(0)
    }

    /// Debug dump: all signal values.
    pub fn dump(&self) -> String {
        self.signals
            .iter()
            .enumerate()
            .map(|(i, &v)| format!("signal[{}]={}", i, v))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The valve controller: programmable inputs + circuit evaluation.
#[derive(Clone, Debug, Default)]
pub struct ValveController {
    pub circuit: LogicCircuit,
}

impl ValveController {
    pub fn new(signal_count: usize) -> Self {
        ValveController {
            circuit: LogicCircuit::new(signal_count),
        }
    }

    pub fn set_input(&mut self, idx: usize, value: Signal) {
        self.circuit.set_input(idx, value);
    }

    pub fn evaluate(&mut self) {
        self.circuit.evaluate();
    }

    pub fn output(&self, idx: usize) -> Signal {
        self.circuit.get(idx)
    }

    /// Debug: all signal states.
    pub fn debug_dump(&self) -> String {
        self.circuit.dump()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AND gate truth table.
    #[test]
    fn p3d701_and_gate() {
        let g = GateKind::And;
        assert_eq!(g.evaluate(&[0, 0]), 0);
        assert_eq!(g.evaluate(&[0, 255]), 0);
        assert_eq!(g.evaluate(&[255, 0]), 0);
        assert_eq!(g.evaluate(&[255, 255]), 255);
    }

    /// OR gate: any high input passes the max.
    #[test]
    fn p3d701_or_gate() {
        let g = GateKind::Or;
        assert_eq!(g.evaluate(&[0, 0]), 0);
        assert_eq!(g.evaluate(&[0, 100]), 100);
        assert_eq!(g.evaluate(&[100, 200]), 200);
    }

    /// NOT gate inverts.
    #[test]
    fn p3d701_not_gate() {
        let g = GateKind::Not;
        assert_eq!(g.evaluate(&[0]), 255);
        assert_eq!(g.evaluate(&[255]), 0);
        assert_eq!(g.evaluate(&[100]), 0, "any positive input inverts to 0");
    }

    /// XOR: odd number of high inputs produces output.
    #[test]
    fn p3d701_xor_gate() {
        let g = GateKind::Xor;
        assert_eq!(g.evaluate(&[0, 0]), 0);
        assert_eq!(g.evaluate(&[100, 0]), 100);
        assert_eq!(g.evaluate(&[100, 100]), 0);
        assert_eq!(g.evaluate(&[100, 100, 100]), 100);
    }

    /// A multi-stage circuit: AND(a, b) → NOT → output.
    #[test]
    fn p3d701_circuit_nand() {
        let mut circuit = LogicCircuit::new(4);
        // Gate 0: AND(signal[0], signal[1]) → signal[2]
        circuit.add_gate("and1", GateKind::And, vec![0, 1], 2);
        // Gate 1: NOT(signal[2]) → signal[3]
        circuit.add_gate("not1", GateKind::Not, vec![2], 3);
        circuit.set_input(0, 255);
        circuit.set_input(1, 255);
        circuit.evaluate();
        assert_eq!(circuit.get(2), 255, "AND output high");
        assert_eq!(circuit.get(3), 0, "NAND output low");
        // Change one input.
        circuit.set_input(1, 0);
        circuit.evaluate();
        assert_eq!(circuit.get(2), 0, "AND output low");
        assert_eq!(circuit.get(3), 255, "NAND output high");
    }

    /// Determinism: same inputs → same outputs, every time.
    #[test]
    fn p3d701_circuit_is_deterministic() {
        let build = || {
            let mut c = ValveController::new(6);
            c.set_input(0, 100);
            c.set_input(1, 50);
            c.circuit.add_gate("or1", GateKind::Or, vec![0, 1], 2);
            c.circuit.add_gate("and1", GateKind::And, vec![0, 2], 3);
            c
        };
        let mut a = build();
        let mut b = build();
        a.evaluate();
        b.evaluate();
        assert_eq!(a.circuit.signals, b.circuit.signals);
    }
}
