//! P3D-613: multi-axis karma — personal, civic, faction, ideological.
//!
//! Each axis accumulates evidence independently. NPC disposition is the
//! weighted sum across axes. Deterministic.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KarmaAxis {
    Personal,
    Civic,
    Faction,
    Ideological,
}

/// Evidence on one axis: weight × confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisEvidence {
    pub axis: KarmaAxis,
    pub weight: i32,
    pub confidence: f32,
}

/// Multi-axis karma for one actor.
#[derive(Clone, Debug, Default)]
pub struct MultiAxisKarma {
    pub axes: std::collections::BTreeMap<KarmaAxis, i32>,
}

impl MultiAxisKarma {
    pub fn record(&mut self, evidence: AxisEvidence) {
        let delta = (evidence.weight as f32 * evidence.confidence).round() as i32;
        *self.axes.entry(evidence.axis).or_insert(0) += delta;
    }

    /// Disposition: weighted sum across all axes, clamped ±100.
    pub fn disposition(&self, axis_weights: &[(KarmaAxis, i32)]) -> i32 {
        let mut total = 0i64;
        let mut total_weight = 0i64;
        for (axis, w) in axis_weights {
            let v = self.axes.get(axis).copied().unwrap_or(0) as i64;
            total += v * (*w as i64);
            total_weight += *w as i64;
        }
        if total_weight == 0 {
            return 0;
        }
        ((total * 100 / total_weight) as i32).clamp(-100, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Multi-axis karma: axes accumulate independently; disposition is
    /// the weighted sum; clamped ±100.
    #[test]
    fn p3d613_multi_axis_karma() {
        let mut k = MultiAxisKarma::default();
        k.record(AxisEvidence {
            axis: KarmaAxis::Personal,
            weight: -10,
            confidence: 1.0,
        });
        k.record(AxisEvidence {
            axis: KarmaAxis::Civic,
            weight: 20,
            confidence: 0.5,
        });
        k.record(AxisEvidence {
            axis: KarmaAxis::Faction,
            weight: -5,
            confidence: 1.0,
        });

        assert_eq!(k.axes[&KarmaAxis::Personal], -10);
        assert_eq!(k.axes[&KarmaAxis::Civic], 10);
        assert_eq!(k.axes[&KarmaAxis::Faction], -5);

        let weights = [
            (KarmaAxis::Personal, 3i32),
            (KarmaAxis::Civic, 2i32),
            (KarmaAxis::Faction, 1i32),
        ];
        let d = k.disposition(&weights);
        assert!(d >= -100 && d <= 100);
    }

    /// Clamping: extreme evidence doesn't exceed ±100.
    #[test]
    fn p3d613_disposition_clamped() {
        let mut k = MultiAxisKarma::default();
        for _ in 0..100 {
            k.record(AxisEvidence {
                axis: KarmaAxis::Personal,
                weight: -100,
                confidence: 1.0,
            });
        }
        let d = k.disposition(&[(KarmaAxis::Personal, 1)]);
        assert_eq!(d, -100);
    }
}
