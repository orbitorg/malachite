use crate::VotingPower;

/// Represents the different quorum thresholds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Threshold<ValueId> {
    /// No quorum has been reached yet
    Unreached,

    /// Quorum of votes but not for the same value
    Any,

    /// Quorum of votes for nil
    Nil,

    /// Quorum (+2/3) of votes for a value
    Value(ValueId),
}

/// Represents the different quorum thresholds.
///
/// There are two thresholds:
/// - The quorum threshold, which is the minimum number of votes required for a quorum.
/// - The honest threshold, which is the minimum number of votes required for a quorum of honest nodes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThresholdParams {
    /// Threshold for a quorum (default: 2f+1)
    pub quorum: ThresholdParam,

    /// Threshold for the minimum number of honest nodes (default: f+1)
    pub honest: ThresholdParam,
}

impl ThresholdParams {
    /// One third of the total weight may be faulty (f = 1/3)
    pub const ONE_THIRD: ThresholdParams = one_third::THRESHOLD_PARAMS;

    /// One fifth of the total weight may be faulty (f = 1/5)
    pub const ONE_FIFTH: ThresholdParams = one_fifth::THRESHOLD_PARAMS;
}

mod one_third {
    use super::{ThresholdParam, ThresholdParams};

    pub const THRESHOLD_PARAMS: ThresholdParams = ThresholdParams {
        quorum: QUORUM,
        honest: HONEST,
    };

    /// More than one third of the total weight (f + 1)
    pub const HONEST: ThresholdParam = ThresholdParam::new(1, 3);

    /// More than two thirds of the total weight (2f + 1)
    pub const QUORUM: ThresholdParam = ThresholdParam::new(2, 3);
}

mod one_fifth {
    use super::{ThresholdParam, ThresholdParams};

    pub const THRESHOLD_PARAMS: ThresholdParams = ThresholdParams {
        quorum: QUORUM,
        honest: HONEST,
    };

    /// More than one fifth of the total weight (f + 1)
    pub const HONEST: ThresholdParam = ThresholdParam::new(1, 5);

    /// More than two fifths of the total weight (2f + 1)
    pub const QUORUM: ThresholdParam = ThresholdParam::new(2, 5);
}

/// Represents the different quorum thresholds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThresholdParam {
    /// Numerator of the threshold
    pub numerator: u64,

    /// Denominator of the threshold
    pub denominator: u64,
}

impl ThresholdParam {
    /// Create a new threshold parameter with the given numerator and denominator.
    pub const fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Check whether the threshold is met.
    pub fn is_met(&self, weight: VotingPower, total: VotingPower) -> bool {
        let lhs = weight
            .checked_mul(self.denominator)
            .expect("attempt to multiply with overflow");

        let rhs = total
            .checked_mul(self.numerator)
            .expect("attempt to multiply with overflow");

        lhs > rhs
    }

    /// Return the minimum expected weight to meet the threshold when applied to the given total.
    pub fn min_expected(&self, total: VotingPower) -> VotingPower {
        total
            .checked_mul(self.numerator)
            .expect("attempt to multiply with overflow")
            .checked_div(self.denominator)
            .expect("attempt to divide with overflow")
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn threshold_param_is_met() {
//         assert!(ThresholdParam::TWO_THIRDS.is_met(7, 10));
//         assert!(!ThresholdParam::TWO_THIRDS.is_met(6, 10));
//         assert!(ThresholdParam::ONE_THIRD.is_met(4, 10));
//         assert!(!ThresholdParam::ONE_THIRD.is_met(3, 10));
//     }
//
//     #[test]
//     #[should_panic(expected = "attempt to multiply with overflow")]
//     fn threshold_param_is_met_overflow() {
//         assert!(!ThresholdParam::TWO_THIRDS.is_met(1, u64::MAX));
//     }
// }
